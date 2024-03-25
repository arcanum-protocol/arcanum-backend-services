use std::{sync::Arc, time::Duration};

use ethers::contract::Multicall;
use ethers::prelude::*;

use futures::{future::join_all, TryFutureExt};
use tokio::{sync::RwLock, task::JoinHandle};

use super::MultipoolWithMeta;

use rpc_controller::RpcRobber;

use multipool::Price;

use anyhow::Result;

const RETRIES: Option<usize> = Some(4);

impl MultipoolWithMeta {
    pub async fn spawn_price_fetching_task(
        multipool: Arc<RwLock<MultipoolWithMeta>>,
        rpc: RpcRobber,
        fetch_interval: u64,
    ) -> JoinHandle<Result<()>> {
        let contract_address = { multipool.read().await.multipool.contract_address() };
        tokio::spawn(async move {
            if let Err(e) = fetch_price(rpc, fetch_interval, contract_address, multipool).await {
                log::error!("Retry limit exceeded for prices, error: {:?}", e);
                std::process::exit(0x69);
            }
            //TODO: remove
            Ok(())
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
        let price_updates = get_prices(&rpc, contract_address, assets, 4).await?;
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
) -> Result<Vec<(Address, Price)>> {
    join_all(assets.chunks(multicall_chunks).map(|assets| {
        rpc.aquire(
            move |provider, multicall_address| async move {
                let mp = multipool_at(contract_address, provider.clone());
                Multicall::new(provider, multicall_address)
                    .await
                    .unwrap()
                    .add_calls(true, assets.iter().map(|asset| mp.get_price(*asset)))
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
                .collect::<Vec<(Address, Price)>>()
        })
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<Vec<(Address, Price)>>>>()
    .map(|data| data.into_iter().flatten().collect())
}
