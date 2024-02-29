use std::{sync::Arc, time::Duration};

use ethers::contract::Multicall;
use ethers::prelude::*;

use futures::{future::join_all, Future, FutureExt, TryFutureExt};
use tokio::sync::RwLock;

use crate::{multipool::Price, rpc_controller::RpcRobber, storage::MultipoolWithMeta};

use anyhow::Result;

const RETRIES: Option<usize> = Some(3);

pub struct PricePoller {
    pub rpc: RpcRobber,
    pub multipool_storage: Arc<RwLock<MultipoolWithMeta>>,
    pub fetch_interval: u64,
}

impl PricePoller {
    pub async fn init(self) -> impl Future<Output = Result<()>> {
        let contract_address = {
            self.multipool_storage
                .read()
                .await
                .multipool
                .contract_address()
        };
        tokio::spawn(fetch_price(
            self.rpc.clone(),
            self.fetch_interval,
            contract_address,
            self.multipool_storage.clone(),
        ))
        .map(|task| match task {
            Err(e) => Err(e.into()),
            Ok(v) => v,
        })
    }
}

pub async fn fetch_price(
    rpc: RpcRobber,
    fetch_interval: u64,
    contract_address: Address,
    multipool_storage: Arc<RwLock<MultipoolWithMeta>>,
) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_millis(fetch_interval));
    loop {
        interval.tick().await;

        let assets = { multipool_storage.read().await.multipool.asset_list() };
        let price_updates =
            get_prices(&rpc, contract_address, assets, 4, MULTICALL_ADDRESS).await?;
        {
            multipool_storage
                .write()
                .await
                .multipool
                .update_prices(&price_updates, false);
        }
    }
}

pub fn multipool_at(
    contract_address: Address,
    provider: Arc<providers::Provider<providers::Http>>,
) -> crate::contracts::multipool::MultipoolContract<providers::Provider<providers::Http>> {
    crate::contracts::multipool::MultipoolContract::new(contract_address, provider)
}

pub async fn get_prices(
    rpc: &RpcRobber,
    contract_address: Address,
    assets: Vec<Address>,
    multicall_chunks: usize,
    multicall_address: Address,
) -> Result<Vec<(Address, Price)>> {
    join_all(assets.chunks(multicall_chunks).into_iter().map(|assets| {
        rpc.aquire(
            move |provider| async move {
                let mp = multipool_at(contract_address, provider.clone());
                Multicall::new(provider, Some(multicall_address))
                    .await
                    .unwrap()
                    .add_calls(
                        true,
                        assets.into_iter().map(|asset| mp.get_price(asset.clone())),
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
                .collect::<Vec<(Address, Price)>>()
        })
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<Vec<(Address, Price)>>>>()
    .map(|data| data.into_iter().flatten().collect())
}
