use std::sync::Arc;

use ethers::prelude::*;

use serde::{Deserialize, Serialize};

use anyhow::Result;
use tokio::sync::RwLock;

use crate::multipool::{
    expiry::MayBeExpired, Multipool, MultipoolAsset, Quantity, QuantityData, Share,
};

use super::{
    MultipoolFactory, MultipoolStorage, MultipoolStorageInner, MultipoolWithMeta, StorageEntry,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Time {
    pub timestamp: u64,
    pub block: U256,
}

impl Time {
    fn new(timestamp: u64, block: U256) -> Self {
        Self { block, timestamp }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FactoryWatcher {
    pub factory_time: Time,
    pub factory_address: Address,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct MultipoolStorageIR {
    pub pools: Vec<MultipoolIR>,
    pub factory_watcher: Option<FactoryWatcher>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultipoolIR {
    pub contract_address: Address,
    pub assets: Vec<MultipoolAssetIR>,
    pub total_supply: Option<Quantity>,
    pub total_shares: Option<Share>,
    pub share_time: Time,
    pub quantity_time: Time,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultipoolAssetIR {
    pub address: Address,
    pub quantity_slot: Option<QuantityDataIr>,
    pub share: Option<Share>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuantityDataIr {
    pub quantity: Quantity,
    pub cashback: Quantity,
}

impl MultipoolStorageIR {
    pub fn try_pack(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(Into::into)
    }

    pub fn try_unpack(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(Into::into)
    }

    pub fn add_factory(mut self, factory_address: Address, start: Time) -> Self {
        self.factory_watcher = Some(FactoryWatcher {
            factory_time: start,
            factory_address,
        });
        self
    }

    pub fn add_pools(mut self, mut pools: Vec<MultipoolIR>) -> Self {
        self.pools.append(&mut pools);
        self
    }

    pub fn build_storage(&self) -> MultipoolStorage {
        MultipoolStorage {
            inner: Arc::new(RwLock::new(MultipoolStorageInner {
                factory: self.factory_watcher.map(|f| MultipoolFactory {
                    factory_time: f.factory_time,
                    factory_address: f.factory_address,
                }),
                pools: self
                    .pools
                    .into_iter()
                    .map(|pool| StorageEntry {
                        address: pool.contract_address,
                        multipool: Arc::new(RwLock::new(MultipoolWithMeta {
                            multipool: Multipool {
                                contract_address: pool.contract_address,
                                assets: pool
                                    .assets
                                    .into_iter()
                                    .map(|asset| MultipoolAsset {
                                        address: asset.address,
                                        price: None,
                                        quantity_slot: asset.quantity_slot.map(
                                            |QuantityDataIr { quantity, cashback }| {
                                                MayBeExpired::with_time(
                                                    QuantityData { quantity, cashback },
                                                    pool.quantity_time.timestamp,
                                                )
                                            },
                                        ),
                                        share: asset.share.map(|s| {
                                            MayBeExpired::with_time(s, pool.share_time.timestamp)
                                        }),
                                    })
                                    .collect(),
                                total_supply: pool.total_supply.map(|v| {
                                    MayBeExpired::with_time(v, pool.quantity_time.timestamp)
                                }),
                                total_shares: pool
                                    .total_shares
                                    .map(|v| MayBeExpired::with_time(v, pool.share_time.timestamp)),
                            },
                            quantity_time: pool.quantity_time,
                            share_time: pool.share_time,
                        })),
                    })
                    .collect(),
            })),
        }
    }
}
