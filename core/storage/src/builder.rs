use std::{pin::Pin, time::Duration};

use futures::{future::join_all, Future, FutureExt};

use multipool_ledger::Ledger;
use rpc_controller::RpcRobber;

use anyhow::Result;

use crate::MultipoolStorage;

use crate::multipool_with_meta::MultipoolWithMeta;

/// Options to build
/// * Use ledger and nothing else (if exists)
/// * Use bootstrap node and save everything to ledger
/// * Use bootstrap node and don't save to ledger
/// * Add additional account
/// * Add Factory address
pub struct MultipoolStorageBuilder<L: Ledger> {
    ledger: Option<L>,
    rpc: Option<RpcRobber>,
    target_share_interval: Option<u64>,
    quantity_interval: Option<u64>,
    sync_interval: Option<u64>,
    price_interval: Option<u64>,
}

impl<L: Ledger> Default for MultipoolStorageBuilder<L> {
    fn default() -> Self {
        Self {
            ledger: None,
            rpc: None,
            target_share_interval: None,
            quantity_interval: None,
            sync_interval: None,
            price_interval: None,
        }
    }
}

impl<L: Ledger + Send + 'static> MultipoolStorageBuilder<L> {
    pub fn ledger(mut self, ledger: L) -> Self {
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

    pub async fn build(self) -> Result<(MultipoolStorage, impl Future)> {
        let ledger = self.ledger.expect("Ledger is not set");
        let storage = MultipoolStorage::from_ir(ledger.read().await?);

        let rpc = self.rpc.expect("Rpc was not set");

        let price_fetch_interval = self.price_interval.expect("Price interval is not set");
        let quantity_fetch_interval = self
            .quantity_interval
            .expect("Quantity interval is not set");
        let target_share_interval = self
            .target_share_interval
            .expect("Target share interval is not set");

        let mut handles = Vec::<Pin<Box<dyn Future<Output = Result<()>>>>>::new();

        let s = storage.inner.read().await;

        for pool in s.pools.iter() {
            let price_handle = MultipoolWithMeta::spawn_price_fetching_task(
                pool.multipool.clone(),
                rpc.clone(),
                price_fetch_interval,
            )
            .await;
            let event_handle = MultipoolWithMeta::spawn_event_fetching_task(
                pool.multipool.clone(),
                rpc.clone(),
                Some(quantity_fetch_interval),
                Some(target_share_interval),
            )
            .await;
            handles.push(price_handle.boxed());
            handles.push(event_handle.boxed());
        }
        drop(s);

        let sync_handle = spawn_syncing_task(
            ledger,
            storage.clone(),
            self.sync_interval.expect("Sync interval is not set"),
        );
        handles.push(sync_handle.boxed());

        let handle = async { join_all(handles).await };
        Ok((storage, handle))
    }
}

pub fn spawn_syncing_task<L: Ledger + Send + 'static>(
    ledger: L,
    storage: MultipoolStorage,
    sync_interval: u64,
) -> impl Future<Output = Result<()>> {
    async move {
        tokio::task::spawn(async move {
            loop {
                let ir = storage.build_ir().await;
                ledger.write(ir)?.await?;
                tokio::time::sleep(Duration::from_millis(sync_interval)).await;
            }
        })
        .await?
    }
}
