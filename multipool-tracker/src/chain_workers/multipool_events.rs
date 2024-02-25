use std::{sync::Arc, time::Duration};

use ethers::contract::Multicall;
use ethers::prelude::*;

use futures::{
    future::{join_all, select},
    TryFutureExt,
};
use tokio::sync::RwLock;

use crate::{
    contracts::multipool::MpAsset,
    multipool::{Multipool, QuantityData, Share},
    rpc_controller::RpcRobber,
};

use anyhow::Result;

const RETRIES: Option<usize> = Some(3);

pub struct EventPoller {
    pub rpc: RpcRobber,
    pub multipool_storage: Arc<RwLock<Multipool>>,
    pub quantity_fetch_interval: u64,
    pub target_share_fetch_interval: u64,
}

impl EventPoller {
    pub async fn init(
        &self,
        initial_assets: Vec<Address>,
        to_fetch_target_shares: bool,
        to_fetch_quantities: bool,
    ) -> Result<()> {
        let contract_address = self.multipool_storage.read().await.contract_address();
        let from_block = current_block(&self.rpc).await?;
        let total_supply = total_supply(&self.rpc, contract_address).await?;
        let assets = get_assets(
            &self.rpc,
            contract_address,
            initial_assets,
            4,
            MULTICALL_ADDRESS,
        )
        .await?;
        {
            let mut mp = self.multipool_storage.write().await;

            let mut quantities = assets
                .iter()
                .map(|(asset_address, quantity_data)| {
                    (
                        asset_address.clone(),
                        QuantityData {
                            cashback: quantity_data.collected_cashbacks.into(),
                            quantity: quantity_data.quantity,
                        },
                    )
                })
                .collect::<Vec<_>>();
            quantities.push((
                contract_address,
                QuantityData {
                    quantity: total_supply,
                    cashback: U256::zero(),
                },
            ));

            mp.update_quantities(&quantities, false);

            mp.update_shares(
                &assets
                    .iter()
                    .map(|(asset_address, quantity_data)| {
                        (asset_address.clone(), quantity_data.target_share.into())
                    })
                    .collect::<Vec<_>>(),
                false,
            );
        }

        let quantity_fetching_future = if to_fetch_quantities {
            tokio::spawn(fetch_quantities(
                self.rpc.clone(),
                self.quantity_fetch_interval,
                contract_address,
                from_block,
                self.multipool_storage.clone(),
            ))
        } else {
            tokio::spawn(futures::future::pending())
        };

        let target_share_fetching_future = if to_fetch_target_shares {
            tokio::spawn(fetch_target_shares(
                self.rpc.clone(),
                self.target_share_fetch_interval,
                contract_address,
                from_block,
                self.multipool_storage.clone(),
            ))
        } else {
            tokio::spawn(futures::future::pending())
        };

        match select(target_share_fetching_future, quantity_fetching_future).await {
            futures::future::Either::Left((v, _)) => v?,
            futures::future::Either::Right((v, _)) => v?,
        }
    }
}

pub async fn fetch_quantities(
    rpc: RpcRobber,
    fetch_interval: u64,
    contract_address: Address,
    start_block: U64,
    multipool_storage: Arc<RwLock<Multipool>>,
) -> Result<()> {
    let mut from_block = start_block;
    let mut interval = tokio::time::interval(Duration::from_millis(fetch_interval));
    loop {
        interval.tick().await;

        let to_block = current_block(&rpc).await?;
        println!("{from_block}, {to_block}");
        let quantity_updates =
            get_quantities_updates(&rpc, contract_address, from_block, to_block).await?;
        multipool_storage
            .write()
            .await
            .update_quantities(&quantity_updates, true);
        from_block = to_block;
    }
}

pub async fn fetch_target_shares(
    rpc: RpcRobber,
    fetch_interval: u64,
    contract_address: Address,
    start_block: U64,
    multipool_storage: Arc<RwLock<Multipool>>,
) -> Result<()> {
    let mut from_block = start_block;
    let mut interval = tokio::time::interval(Duration::from_millis(fetch_interval));
    loop {
        interval.tick().await;

        let to_block = current_block(&rpc).await?;
        let target_shares_updates =
            get_target_shares_updates(&rpc, contract_address, from_block, to_block).await?;
        multipool_storage
            .write()
            .await
            .update_shares(&target_shares_updates, true);
        from_block = to_block;
    }
}

pub async fn current_block(rpc: &RpcRobber) -> Result<U64> {
    rpc.aquire(
        |provider| async move { provider.get_block_number().await.map_err(|e| e.into()) },
        RETRIES,
    )
    .await
}

pub fn multipool_at(
    contract_address: Address,
    provider: Arc<providers::Provider<providers::Http>>,
) -> crate::contracts::multipool::MultipoolContract<providers::Provider<providers::Http>> {
    crate::contracts::multipool::MultipoolContract::new(contract_address, provider)
}

pub async fn total_supply(rpc: &RpcRobber, contract_address: Address) -> Result<U256> {
    rpc.aquire(
        |provider| async move {
            multipool_at(contract_address, provider)
                .total_supply()
                .await
                .map_err(|e| e.into())
        },
        RETRIES,
    )
    .await
}

pub async fn total_target_shares(rpc: &RpcRobber, contract_address: Address) -> Result<U256> {
    rpc.aquire(
        |provider| async move {
            multipool_at(contract_address, provider)
                .total_target_shares()
                .await
                .map_err(|e| e.into())
        },
        RETRIES,
    )
    .await
}

pub async fn get_assets(
    rpc: &RpcRobber,
    contract_address: Address,
    assets: Vec<Address>,
    multicall_chunks: usize,
    multicall_address: Address,
) -> Result<Vec<(Address, MpAsset)>> {
    join_all(assets.chunks(multicall_chunks).into_iter().map(|assets| {
        rpc.aquire(
            move |provider| async move {
                let mp = multipool_at(contract_address, provider.clone());
                Multicall::new(provider, Some(multicall_address))
                    .await
                    .unwrap()
                    .add_calls(
                        true,
                        assets.into_iter().map(|asset| mp.get_asset(asset.clone())),
                    )
                    .call_array()
                    .await
                    .map_err(Into::into)
            },
            RETRIES,
        )
        .map_ok(|data| {
            assets
                .into_iter()
                .cloned()
                .zip(data)
                .collect::<Vec<(Address, MpAsset)>>()
        })
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<Vec<(Address, MpAsset)>>>>()
    .map(|data| data.into_iter().flatten().collect())
}

pub async fn get_quantities_updates(
    rpc: &RpcRobber,
    contract_address: Address,
    from_block: U64,
    to_block: U64,
) -> Result<Vec<(Address, QuantityData)>> {
    rpc.aquire(
        move |provider| async move {
            multipool_at(contract_address, provider.clone())
                .asset_change_filter()
                .from_block(from_block)
                .to_block(to_block)
                .query()
                .await
                .map_err(Into::into)
        },
        RETRIES,
    )
    .await
    .map(|logs| {
        logs.into_iter()
            .map(|log| {
                (
                    log.asset,
                    QuantityData {
                        quantity: log.quantity,
                        cashback: U256::from(log.collected_cashbacks),
                    },
                )
            })
            .collect()
    })
}

pub async fn get_target_shares_updates(
    rpc: &RpcRobber,
    contract_address: Address,
    from_block: U64,
    to_block: U64,
) -> Result<Vec<(Address, Share)>> {
    rpc.aquire(
        move |provider| async move {
            multipool_at(contract_address, provider.clone())
                .target_share_change_filter()
                .from_block(from_block)
                .to_block(to_block)
                .query()
                .await
                .map_err(Into::into)
        },
        RETRIES,
    )
    .await
    .map(|logs| {
        logs.into_iter()
            .map(|log| (log.asset, log.new_target_share))
            .collect()
    })
}
