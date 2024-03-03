use std::{sync::Arc, time::Duration};

use anyhow::Result;
use ethers::prelude::*;
use rpc_controller::RpcRobber;
use tokio::{sync::RwLock, task::JoinHandle};

use crate::MultipoolStorage;

const RETRIES: Option<usize> = Some(3);

#[derive(Debug)]
pub struct FactoryWatcher {
    pub factory_address: Address,
    pub block_number: RwLock<U64>,
}

#[derive(Clone)]
pub struct IntervalParams {
    pub price_fetch_interval: u64,
    pub quantity_fetch_interval: Option<u64>,
    pub target_share_fetch_interval: Option<u64>,
}

impl FactoryWatcher {
    pub fn new(factory_address: Address, block_number: U64) -> Self {
        Self {
            factory_address,
            block_number: RwLock::new(block_number),
        }
    }

    pub async fn spawn_new_multipool_monitoring_task(
        factory_watcher: Arc<FactoryWatcher>,
        multipool_storage: MultipoolStorage,
        rpc: RpcRobber,
        monitoring_interval: u64,
        intervals: IntervalParams,
    ) -> JoinHandle<Result<()>> {
        tokio::spawn(fetch_spawn_events(
            rpc.clone(),
            monitoring_interval,
            multipool_storage,
            factory_watcher,
            intervals,
        ))
    }
}

pub async fn fetch_spawn_events(
    rpc: RpcRobber,
    fetch_interval: u64,
    multipool_storage: MultipoolStorage,
    factory_watcher: Arc<FactoryWatcher>,
    intervals: IntervalParams,
) -> Result<()> {
    let mut from_block = factory_watcher.block_number.read().await.to_owned();
    let mut interval = tokio::time::interval(Duration::from_millis(fetch_interval));
    let contract_address = factory_watcher.factory_address;
    loop {
        interval.tick().await;

        let to_block = current_block(&rpc).await?;
        let new_multipools =
            get_spawned_multipools(&rpc, contract_address, from_block, to_block).await?;
        {
            // Insert pools then assign block so it something failes in the middle existing pools
            // will be re-indexed and no data will be lost
            multipool_storage
                .insert_new_pools(new_multipools.as_slice(), &rpc, &intervals)
                .await;
            let mut block = factory_watcher.block_number.write().await;
            *block = to_block;
            drop(block);
        }
        from_block = to_block + 1;
    }
}

pub async fn current_block(rpc: &RpcRobber) -> Result<U64> {
    rpc.aquire(
        |provider, _| async move { provider.get_block_number().await.map_err(|e| e.into()) },
        RETRIES,
    )
    .await
}

pub fn multipool_factory_at(
    contract_address: Address,
    provider: Arc<providers::Provider<providers::Http>>,
) -> crate::contracts::multipool_factory::MultipoolFactoryContract<
    providers::Provider<providers::Http>,
> {
    crate::contracts::multipool_factory::MultipoolFactoryContract::new(contract_address, provider)
}

pub async fn get_spawned_multipools(
    rpc: &RpcRobber,
    contract_address: Address,
    from_block: U64,
    to_block: U64,
) -> Result<Vec<(Address, U64)>> {
    rpc.aquire(
        move |provider, _| async move {
            multipool_factory_at(contract_address, provider.clone())
                .multipool_spawned_filter()
                .from_block(from_block)
                .to_block(to_block)
                .query()
                .await
                .map_err(Into::into)
        },
        RETRIES,
    )
    .await
    //TODO: change to multipool_address
    .map(|logs| logs.into_iter().map(|log| (log.p0, from_block)).collect())
}
