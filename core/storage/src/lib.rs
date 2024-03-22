pub mod builder;
pub mod contracts;
pub mod factory_watcher;
pub mod ir_builder;
pub mod multipool_with_meta;

#[cfg(test)]
pub mod tests;

use std::sync::Arc;

use anyhow::Result;
use ethers::prelude::*;
use factory_watcher::{FactoryWatcher, IntervalParams};
use multipool_with_meta::MultipoolWithMeta;
use rpc_controller::RpcRobber;
use tokio::{sync::RwLock, task::JoinHandle};

pub trait MultipoolStorageHook: Send + Sync {
    fn new_pool(&self, pool: Arc<RwLock<MultipoolWithMeta>>);
}

#[derive(Debug, Default)]
pub struct MultipoolStorage<H: MultipoolStorageHook> {
    inner: Arc<RwLock<MultipoolStorageInner<H>>>,
}

impl MultipoolStorageHook for () {
    fn new_pool(&self, _pool: Arc<RwLock<MultipoolWithMeta>>) {}
}

impl<H: MultipoolStorageHook> Clone for MultipoolStorage<H> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct MultipoolStorageInner<H: MultipoolStorageHook> {
    pub pools: Vec<StorageEntry>,
    pub factories: Vec<Arc<FactoryWatcher>>,
    pub handles: Vec<JoinHandle<Result<()>>>,
    pub hook: Option<H>,
}

#[derive(Debug, Clone)]
pub struct StorageEntry {
    pub multipool: Arc<RwLock<MultipoolWithMeta>>,
    pub address: Address,
}

impl StorageEntry {
    pub async fn new(pool: MultipoolWithMeta) -> Result<Self> {
        let mp = StorageEntry {
            address: pool.multipool.contract_address(),
            multipool: Arc::new(RwLock::new(pool)),
        };
        Ok(mp)
    }
}

impl<H: MultipoolStorageHook + Send + Sync + 'static> MultipoolStorage<H> {
    pub async fn insert_new_pools(
        &self,
        pools: &[(Address, U64)],
        rpc: &RpcRobber,
        intervals: &IntervalParams,
    ) {
        let mut new_pools = Vec::new();
        for (address, block_number) in pools {
            let mp = Arc::new(RwLock::new(MultipoolWithMeta::new(*address, *block_number)));
            new_pools.push(mp.clone());
            let entry = StorageEntry {
                multipool: mp.clone(),
                address: *address,
            };
            self.inner.write().await.pools.push(entry);
            if let Some(ref h) = self.inner.read().await.hook {
                h.new_pool(mp);
            }
        }
        self.run_multipool_tasks(new_pools.as_slice(), rpc, intervals)
            .await;
    }

    pub async fn abort_handles(&self) {
        self.inner
            .write()
            .await
            .handles
            .iter()
            .for_each(|h| h.abort());
    }

    pub async fn append_handle(&self, handle: JoinHandle<Result<()>>) {
        self.inner.write().await.handles.push(handle);
    }

    pub async fn run_factory_tasks(
        &self,
        factories: &[Arc<FactoryWatcher>],
        rpc: &RpcRobber,
        monitor_interval: u64,
        intervals: &IntervalParams,
    ) {
        for factory in factories {
            let factory_handle = FactoryWatcher::spawn_new_multipool_monitoring_task(
                factory.clone(),
                (*self).clone(),
                rpc.clone(),
                monitor_interval,
                intervals.clone(),
            )
            .await;
            self.inner.write().await.handles.push(factory_handle);
        }
    }

    pub async fn run_multipool_tasks(
        &self,
        pools: &[Arc<RwLock<MultipoolWithMeta>>],
        rpc: &RpcRobber,
        intervals: &IntervalParams,
    ) {
        for pool in pools {
            let price_handle = MultipoolWithMeta::spawn_price_fetching_task(
                pool.clone(),
                rpc.clone(),
                intervals.price_fetch_interval,
            )
            .await;
            let (quantity_handle, target_share_handle) =
                MultipoolWithMeta::spawn_event_fetching_task(
                    pool.clone(),
                    rpc.clone(),
                    intervals.quantity_fetch_interval,
                    intervals.target_share_fetch_interval,
                )
                .await;
            if let Some(ref h) = self.inner.read().await.hook {
                h.new_pool(pool.clone());
            }
            self.inner.write().await.handles.extend([
                price_handle,
                quantity_handle,
                target_share_handle,
            ]);
        }
    }

    pub async fn get_pool(&self, address: &Address) -> Option<Arc<RwLock<MultipoolWithMeta>>> {
        self.inner
            .read()
            .await
            .pools
            .iter()
            .find(|p| p.address.eq(address))
            .map(|p| p.multipool.clone())
    }

    pub async fn pools(&self) -> Vec<StorageEntry> {
        self.inner.read().await.pools.clone()
    }
}
