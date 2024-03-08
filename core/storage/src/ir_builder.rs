use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use futures::future::join_all;
use multipool::expiry::MayBeExpired;
use multipool::Multipool;
use multipool::MultipoolAsset;
use multipool::QuantityData;
use multipool_ledger::ir::MultipoolAssetIR;
use multipool_ledger::ir::MultipoolFactoryIR;
use multipool_ledger::ir::MultipoolIR;
use multipool_ledger::ir::MultipoolStorageIR;
use multipool_ledger::ir::QuantityDataIr;

use ethers::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::factory_watcher::FactoryWatcher;
use crate::multipool_with_meta::MultipoolWithMeta;
use crate::MultipoolStorage;
use crate::MultipoolStorageInner;
use crate::StorageEntry;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ExternalMultipool {
    pub contract_address: Address,
    pub assets: Vec<Address>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ExternalFactory {
    pub factory_address: Address,
    pub block_number: u64,
}

impl ExternalMultipool {
    pub fn read_pools(path: PathBuf) -> Vec<Self> {
        serde_yaml::from_slice(fs::read(path).expect("Config should exist").as_slice())
            .expect("Config should be valid")
    }
}

pub trait MultipoolStorageIRBuilder: Sized {
    fn add_pool(self, pool: MultipoolWithMeta) -> Option<Self>;
    fn add_factory(self, factory: ExternalFactory) -> Option<Self>;
}

impl MultipoolStorageIRBuilder for MultipoolStorageIR {
    fn add_pool(mut self, pool: MultipoolWithMeta) -> Option<Self> {
        if self
            .pools
            .iter()
            .find(|p| p.contract_address.eq(&pool.multipool.contract_address))
            .is_some()
        {
            return None;
        }
        self.pools.push(build_multipool_ir(pool));
        Some(self)
    }

    fn add_factory(mut self, factory: ExternalFactory) -> Option<Self> {
        if self
            .factories
            .iter()
            .find(|f| f.factory_address.eq(&factory.factory_address))
            .is_some()
        {
            return None;
        }
        self.factories.push(MultipoolFactoryIR {
            factory_block: factory.block_number,
            factory_address: factory.factory_address,
        });
        Some(self)
    }
}

fn build_multipool_ir(pool: MultipoolWithMeta) -> MultipoolIR {
    let MultipoolWithMeta {
        multipool,
        quantity_time,
        share_time,
    } = pool;
    MultipoolIR {
        contract_address: multipool.contract_address,
        assets: multipool
            .assets
            .into_iter()
            .map(|a| MultipoolAssetIR {
                address: a.address,
                quantity_slot: a.quantity_slot.map(MayBeExpired::any_age).map(
                    |QuantityData { quantity, cashback }| QuantityDataIr { quantity, cashback },
                ),
                share: a.share.map(MayBeExpired::any_age),
            })
            .collect(),
        total_supply: multipool.total_supply.map(|v| v.any_age()),
        total_shares: multipool.total_shares.map(|v| v.any_age()),
        share_time,
        quantity_time,
    }
}

impl MultipoolStorage {
    pub async fn build_ir(&self) -> MultipoolStorageIR {
        let value = self.inner.read().await;
        let pools = value.pools.clone();
        let factories = value.factories.clone();
        drop(value);

        let pools = join_all(pools.into_iter().map(|pool| async move {
            let mp = pool.multipool.read().await;
            let val = mp.to_owned();
            drop(mp);
            val
        }))
        .await
        .into_iter()
        .map(build_multipool_ir)
        .collect();
        let factories = join_all(factories.into_iter().map(|factory| async move {
            MultipoolFactoryIR {
                factory_block: factory.block_number.read().await.clone().as_u64(),
                factory_address: factory.factory_address,
            }
        }))
        .await;
        MultipoolStorageIR { pools, factories }
    }

    pub fn from_ir(ir: MultipoolStorageIR) -> MultipoolStorage {
        MultipoolStorage {
            inner: Arc::new(RwLock::new(MultipoolStorageInner {
                handles: Default::default(),
                factories: ir
                    .factories
                    .into_iter()
                    .map(|f| {
                        Arc::new(FactoryWatcher::new(
                            f.factory_address,
                            f.factory_block.into(),
                        ))
                    })
                    .collect(),
                pools: ir
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
