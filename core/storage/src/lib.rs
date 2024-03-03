pub mod builder;
pub mod contracts;
pub mod ir_builder;
pub mod multipool_with_meta;

#[cfg(test)]
pub mod tests;

use std::sync::Arc;

use anyhow::Result;
use ethers::prelude::*;
use multipool_with_meta::MultipoolWithMeta;
use tokio::sync::RwLock;

use multipool_ledger::ir::Time;

#[derive(Debug, Default, Clone)]
pub struct MultipoolStorage {
    inner: Arc<RwLock<MultipoolStorageInner>>,
}

#[derive(Debug, Clone, Default)]
pub struct MultipoolStorageInner {
    pub pools: Vec<StorageEntry>,
    pub factories: Vec<MultipoolFactory>,
}

#[derive(Debug, Clone)]
pub struct MultipoolFactory {
    pub factory_time: Time,
    pub factory_address: Address,
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

impl MultipoolStorage {
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
