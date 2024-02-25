pub mod ir;
pub mod ledger;

use std::sync::Arc;

use ethers::prelude::*;
use tokio::sync::RwLock;

use crate::multipool::Multipool;

use self::ir::Time;

#[derive(Debug)]
pub struct MultipoolStorage {
    inner: Arc<RwLock<MultipoolStorageInner>>,
}

#[derive(Debug)]
pub struct MultipoolStorageInner {
    pub pools: Vec<StorageEntry>,
    pub factory: Option<MultipoolFactory>,
}

#[derive(Debug)]
pub struct MultipoolFactory {
    pub factory_time: Time,
    pub factory_address: Address,
}

#[derive(Debug)]
pub struct StorageEntry {
    multipool: Arc<RwLock<MultipoolWithMeta>>,
    address: Address,
}

#[derive(Debug)]
pub struct MultipoolWithMeta {
    multipool: Multipool,
    quantity_time: Time,
    share_time: Time,
}

impl MultipoolStorage {
    pub fn get_pool(&self, address: Address) -> Arc<RwLock<Multipool>> {
        unimplemented!()
    }

    pub fn from_ir() -> Self {
        todo!();
    }

    pub fn build_ir(&self) -> MultipoolStorage {
        todo!("Build storage from ir")
    }
}
