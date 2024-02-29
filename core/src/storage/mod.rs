pub mod builder;
pub mod ir;
pub mod ledger;

use std::{fs, path::PathBuf, sync::Arc};

use anyhow::Result;
use ethers::prelude::*;
use futures::{future::join_all, Future};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    chain_workers::{multipool_events::EventPoller, multipool_prices::PricePoller},
    multipool::{expiry::MayBeExpired, Multipool, QuantityData},
    rpc_controller::RpcRobber,
};

use self::ir::{
    MultipoolAssetIR, MultipoolFactoryIR, MultipoolIR, MultipoolStorageIR, QuantityDataIr, Time,
};

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
    pub async fn init_from_external(pool: ExternalMultipool, rpc: RpcRobber) -> Result<Self> {
        let mp = StorageEntry {
            address: pool.contract_address,
            multipool: Arc::new(RwLock::new(MultipoolWithMeta {
                multipool: Multipool::new(pool.contract_address),
                quantity_time: Time::new(0.into()),
                share_time: Time::new(0.into()),
            })),
        };
        MultipoolWithMeta::fill_multipool(&mp.multipool, &rpc, pool.assets).await?;
        Ok(mp)
    }
}

#[derive(Debug, Clone)]
pub struct MultipoolWithMeta {
    pub multipool: Multipool,
    pub quantity_time: Time,
    pub share_time: Time,
}

impl MultipoolWithMeta {
    pub async fn spawn_price_fetching_task(
        multipool: &Arc<RwLock<Self>>,
        rpc: &RpcRobber,
        interval: u64,
    ) -> impl Future<Output = Result<()>> {
        let rpc = rpc.clone();
        let multipool = multipool.clone();
        PricePoller {
            rpc,
            multipool_storage: multipool,
            fetch_interval: interval,
        }
        .init()
        .await
    }

    /// Don't start routine if interval is none
    pub async fn spawn_event_fetching_task(
        multipool: &Arc<RwLock<Self>>,
        rpc: &RpcRobber,
        quantity_interval: Option<u64>,
        target_share_interval: Option<u64>,
    ) -> impl Future<Output = Result<()>> {
        let rpc = rpc.clone();
        let multipool = multipool.clone();
        EventPoller {
            rpc,
            multipool_storage: multipool,
        }
        .init(quantity_interval, target_share_interval)
        .await
    }

    pub async fn fill_multipool(
        multipool: &Arc<RwLock<Self>>,
        rpc: &RpcRobber,
        initial_assets: Vec<Address>,
    ) -> Result<()> {
        let rpc = rpc.clone();
        let multipool = multipool.clone();
        EventPoller {
            rpc,
            multipool_storage: multipool,
        }
        .fill(initial_assets)
        .await
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ExternalMultipool {
    contract_address: Address,
    assets: Vec<Address>,
}

impl ExternalMultipool {
    pub fn read_pools(path: PathBuf) -> Vec<Self> {
        serde_yaml::from_slice(fs::read(path).expect("Config should exist").as_slice())
            .expect("Config should be valid")
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

    pub async fn build_ir(&self) -> MultipoolStorageIR {
        let value = self.inner.read().await.to_owned();
        let pools = join_all(value.pools.into_iter().map(|pool| async move {
            let mp = pool.multipool.read().await;
            let val = mp.to_owned();
            drop(mp);
            val
        }))
        .await
        .into_iter()
        .map(
            |MultipoolWithMeta {
                 multipool,
                 quantity_time,
                 share_time,
             }| MultipoolIR {
                contract_address: multipool.contract_address,
                assets: multipool
                    .assets
                    .into_iter()
                    .map(|a| MultipoolAssetIR {
                        address: a.address,
                        quantity_slot: a.quantity_slot.map(MayBeExpired::any_age).map(
                            |QuantityData { quantity, cashback }| QuantityDataIr {
                                quantity,
                                cashback,
                            },
                        ),
                        share: a.share.map(MayBeExpired::any_age),
                    })
                    .collect(),
                total_supply: multipool.total_supply.map(|v| v.any_age()),
                total_shares: multipool.total_shares.map(|v| v.any_age()),
                share_time,
                quantity_time,
            },
        )
        .collect();
        let factories = value
            .factories
            .into_iter()
            .map(|f| MultipoolFactoryIR {
                factory_time: f.factory_time,
                factory_address: f.factory_address,
            })
            .collect();
        MultipoolStorageIR { pools, factories }
    }
}
