use std::pin::Pin;

use futures::{future::join_all, Future, FutureExt};

use crate::rpc_controller::RpcRobber;

use anyhow::Result;

use super::{
    ledger::Ledger, ExternalMultipool, MultipoolFactory, MultipoolStorage, MultipoolWithMeta,
    StorageEntry,
};

/// Options to build
/// * Use ledger and nothing else (if exists)
/// * Use bootstrap node and save everything to ledger
/// * Use bootstrap node and don't save to ledger
/// * Add additional account
/// * Add Factory address
#[derive(Default)]
pub struct MultipoolStorageBuilder {
    ledger: Option<Ledger>,
    rpc: Option<RpcRobber>,
    target_share_interval: Option<u64>,
    quantity_interval: Option<u64>,
    sync_interval: Option<u64>,
    price_interval: Option<u64>,
    external_pools: Vec<ExternalMultipool>,
    //bootstrap: Option<()>,
    factories: Vec<MultipoolFactory>,
}

impl MultipoolStorageBuilder {
    pub fn ledger(mut self, ledger: Ledger) -> Self {
        self.ledger = Some(ledger);
        self
    }

    pub fn rpc(mut self, rpc: RpcRobber) -> Self {
        self.rpc = Some(rpc);
        self
    }

    pub fn target_share_interval(mut self, interval: u64) -> Self {
        self.target_share_interval = Some(interval);
        self
    }

    pub fn ledger_sync_interval(mut self, interval: u64) -> Self {
        self.sync_interval = Some(interval);
        self
    }

    pub fn quantity_interval(mut self, interval: u64) -> Self {
        self.quantity_interval = Some(interval);
        self
    }

    pub fn price_interval(mut self, interval: u64) -> Self {
        self.price_interval = Some(interval);
        self
    }

    pub fn with_pools(mut self, mut pools: Vec<ExternalMultipool>) -> Self {
        self.external_pools.append(&mut pools);
        self
    }

    pub fn with_factories(mut self, mut factories: Vec<MultipoolFactory>) -> Self {
        self.factories.append(&mut factories);
        self
    }

    pub async fn build(mut self) -> Result<(MultipoolStorage, impl Future)> {
        let storage = if let Some(ledger) = self.ledger.as_ref() {
            ledger.read().await?.build_storage()
        } else {
            MultipoolStorage::default()
        };
        let rpc = self.rpc.expect("Rpc was not set");
        let mut s = storage.inner.write().await;
        s.factories.append(&mut self.factories);
        for pool in self.external_pools {
            let mp = StorageEntry::init_from_external(pool, rpc.clone()).await?;
            s.pools.push(mp);
        }
        let price_fetch_interval = self.price_interval.expect("Price interval is not set");
        let quantity_fetch_interval = self
            .quantity_interval
            .expect("Quantity interval is not set");
        let target_share_interval = self
            .target_share_interval
            .expect("Target share interval is not set");

        let mut handles = Vec::<Pin<Box<dyn Future<Output = Result<()>>>>>::new();

        for pool in s.pools.iter() {
            let price_handle = MultipoolWithMeta::spawn_price_fetching_task(
                &pool.multipool,
                &rpc,
                price_fetch_interval,
            )
            .await;
            let event_handle = MultipoolWithMeta::spawn_event_fetching_task(
                &pool.multipool,
                &rpc,
                Some(quantity_fetch_interval),
                Some(target_share_interval),
            )
            .await;
            handles.push(price_handle.boxed());
            handles.push(event_handle.boxed());
        }
        drop(s);

        if let Some(ledger) = self.ledger {
            let sync_handle = ledger.spawn_syncing_task(
                storage.clone(),
                self.sync_interval.expect("Sync interval is not set"),
            );
            handles.push(sync_handle.boxed());
        }

        let handle = async { join_all(handles).await };
        Ok((storage, handle))
    }
}
