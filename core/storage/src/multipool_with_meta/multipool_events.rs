use std::{sync::Arc, time::Duration};

use ethers::contract::Multicall;
use ethers::prelude::*;

use futures::{future::join_all, TryFutureExt};
use tokio::{sync::RwLock, task::JoinHandle};

use crate::{contracts::multipool::MpAsset, ir_builder::ExternalMultipool};

use super::MultipoolWithMeta;

use multipool_ledger::ir::Time;

use rpc_controller::RpcRobber;

use multipool::{Multipool, QuantityData, Share};

use anyhow::Result;

const RETRIES: Option<usize> = Some(1000);

impl MultipoolWithMeta {
    pub async fn spawn_event_fetching_task(
        multipool: Arc<RwLock<MultipoolWithMeta>>,
        rpc: RpcRobber,
        quantity_fetch_interval: Option<u64>,
        target_share_fetch_interval: Option<u64>,
    ) -> (JoinHandle<Result<()>>, JoinHandle<Result<()>>) {
        let mp = multipool.read().await;
        let contract_address = mp.multipool.contract_address();
        let quantity_from_block = mp.quantity_time.block;
        let share_from_block = mp.share_time.block;
        drop(mp);

        let quantity_fetching_future = if let Some(interval) = quantity_fetch_interval {
            let rpc = rpc.clone();
            let multipool = multipool.clone();
            tokio::spawn(async move {
                if let Err(e) = fetch_quantities(
                    rpc,
                    interval,
                    contract_address,
                    quantity_from_block,
                    multipool,
                )
                .await
                {
                    log::error!("Retry limit exceeded for quantities, error: {:?}", e);
                    std::process::exit(0x69);
                }
                //TODO: remove
                Ok(())
            })
        } else {
            tokio::spawn(futures::future::pending())
        };

        let target_share_fetching_future = if let Some(interval) = target_share_fetch_interval {
            tokio::spawn(async move {
                if let Err(e) = fetch_target_shares(
                    rpc,
                    interval,
                    contract_address,
                    share_from_block,
                    multipool,
                )
                .await
                {
                    log::error!("Retry limit exceeded for target shares, error: {:?}", e);
                    std::process::exit(0x69);
                }
                //TODO: remove
                Ok(())
            })
        } else {
            tokio::spawn(futures::future::pending())
        };

        (quantity_fetching_future, target_share_fetching_future)
    }

    pub fn new(address: Address, start_block: U64) -> Self {
        Self {
            multipool: Multipool::new(address),
            quantity_time: Time::new(start_block),
            share_time: Time::new(start_block),
        }
    }

    pub async fn fill(external_multipool: ExternalMultipool, rpc: &RpcRobber) -> Result<Self> {
        let contract_address = external_multipool.contract_address;
        let from_block = current_block(rpc).await?;
        let total_supply = total_supply(rpc, contract_address).await?;
        let assets = get_assets(rpc, contract_address, external_multipool.assets, 4).await?;

        let mut storage = MultipoolWithMeta {
            multipool: Multipool::new(contract_address),
            quantity_time: Time::new(from_block),
            share_time: Time::new(from_block),
        };

        {
            let mp = &mut storage.multipool;

            let mut quantities = assets
                .iter()
                .map(|(asset_address, quantity_data)| {
                    (
                        *asset_address,
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
                        (*asset_address, quantity_data.target_share.into())
                    })
                    .collect::<Vec<_>>(),
                false,
            );
        }
        Ok(storage)
    }
}

pub async fn fetch_quantities(
    rpc: RpcRobber,
    fetch_interval: u64,
    contract_address: Address,
    start_block: U64,
    multipool_storage: Arc<RwLock<MultipoolWithMeta>>,
) -> Result<()> {
    let mut from_block = start_block;
    let mut interval = tokio::time::interval(Duration::from_millis(fetch_interval));
    loop {
        interval.tick().await;

        let (quantity_updates, to_block) =
            get_quantities_updates(&rpc, contract_address, from_block).await?;
        {
            let mut mp = multipool_storage.write().await;
            mp.multipool.update_quantities(&quantity_updates, true);
            mp.quantity_time = Time::new(to_block);
            drop(mp);
        }
        from_block = to_block;
    }
}

pub async fn fetch_target_shares(
    rpc: RpcRobber,
    fetch_interval: u64,
    contract_address: Address,
    start_block: U64,
    multipool_storage: Arc<RwLock<MultipoolWithMeta>>,
) -> Result<()> {
    let mut from_block = start_block;
    let mut interval = tokio::time::interval(Duration::from_millis(fetch_interval));
    loop {
        interval.tick().await;

        let (target_shares_updates, to_block) =
            get_target_shares_updates(&rpc, contract_address, from_block).await?;
        {
            let mut mp = multipool_storage.write().await;
            mp.multipool.update_shares(&target_shares_updates, true);
            mp.share_time = Time::new(to_block);
            drop(mp);
        }
        from_block = to_block;
    }
}

pub async fn current_block(rpc: &RpcRobber) -> Result<U64> {
    rpc.aquire(
        |provider, _| async move { provider.get_block_number().await.map_err(|e| e.into()) },
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
        |provider, _| async move {
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
        |provider, _| async move {
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
) -> Result<Vec<(Address, MpAsset)>> {
    join_all(assets.chunks(multicall_chunks).map(|assets| {
        rpc.aquire(
            move |provider, multicall_address| async move {
                let mp = multipool_at(contract_address, provider.clone());
                Multicall::new(provider, multicall_address)
                    .await
                    .unwrap()
                    .add_calls(true, assets.iter().map(|asset| mp.get_asset(*asset)))
                    .call_array()
                    .await
                    .map_err(Into::into)
            },
            RETRIES,
        )
        .map_ok(|data| {
            assets
                .iter()
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
) -> Result<(Vec<(Address, QuantityData)>, U64)> {
    rpc.aquire(
        move |provider, _| async move {
            //TODO: change to declarative
            let to_block = provider.get_block_number().await?;
            let events = multipool_at(contract_address, provider.clone())
                .asset_change_filter()
                .from_block(from_block)
                .to_block(to_block - 1)
                .query()
                .await?;
            Ok((events, to_block))
        },
        RETRIES,
    )
    .await
    .map(|(logs, block)| {
        let logs = logs
            .into_iter()
            .map(|log| {
                (
                    log.asset,
                    QuantityData {
                        quantity: log.quantity,
                        cashback: U256::from(log.collected_cashbacks),
                    },
                )
            })
            .collect();
        (logs, block)
    })
}

pub async fn get_target_shares_updates(
    rpc: &RpcRobber,
    contract_address: Address,
    from_block: U64,
) -> Result<(Vec<(Address, Share)>, U64)> {
    rpc.aquire(
        move |provider, _| async move {
            let to_block = provider.get_block_number().await?;
            let events = multipool_at(contract_address, provider.clone())
                .target_share_change_filter()
                .from_block(from_block)
                .to_block(to_block - 1)
                .query()
                .await?;
            Ok((events, to_block))
        },
        RETRIES,
    )
    .await
    .map(|(logs, to_block)| {
        let logs = logs
            .into_iter()
            .map(|log| (log.asset, log.new_target_share))
            .collect();
        (logs, to_block)
    })
}
