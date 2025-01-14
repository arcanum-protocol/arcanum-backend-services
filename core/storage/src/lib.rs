use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use alloy::primitives::Address;
use anyhow::Result;
use multipool::{expiry::StdTimeExtractor, Multipool};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, task::JoinHandle};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Time {
    pub timestamp: u64,
    pub block: u64,
}

impl Time {
    pub fn new(block: u64) -> Self {
        Self {
            block,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Shold be always after epoch start")
                .as_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipoolWithMeta {
    pub multipool: Multipool<StdTimeExtractor>,
    pub quantity_time: Time,
    pub share_time: Time,
}

impl MultipoolWithMeta {
    pub fn new(address: Address, start_block: u64) -> Self {
        Self {
            multipool: Multipool::new(address),
            quantity_time: Time::new(start_block),
            share_time: Time::new(start_block),
        }
    }
}

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
    pub async fn insert_new_pools(&self, pools: &[(Address, u64)]) {
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
