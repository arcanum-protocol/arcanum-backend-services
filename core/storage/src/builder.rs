use std::time::Duration;

use multipool_ledger::Ledger;
use rpc_controller::RpcRobber;

use anyhow::Result;
use tokio::task::JoinHandle;

use crate::MultipoolStorage;

use crate::factory_watcher::IntervalParams;

pub struct MultipoolStorageBuilder<L: Ledger> {
    ledger: Option<L>,
    rpc: Option<RpcRobber>,
    target_share_interval: Option<u64>,
    quantity_interval: Option<u64>,
    sync_interval: Option<u64>,
    price_interval: Option<u64>,
    monitoring_interval: Option<u64>,
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
            monitoring_interval: None,
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

    pub fn monitoring_interval(mut self, interval: u64) -> Self {
        self.monitoring_interval = Some(interval);
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

    pub async fn build(self) -> Result<MultipoolStorage> {
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
        let monitoring_intgerval = self
            .monitoring_interval
            .expect("Monitoring interval is not set");
        let interval_params = IntervalParams {
            price_fetch_interval,
            quantity_fetch_interval: Some(quantity_fetch_interval),
            target_share_fetch_interval: Some(target_share_interval),
        };

        let pools = storage.inner.read().await.pools.clone();
        storage
            .run_multipool_tasks(
                pools
                    .iter()
                    .map(|e| e.multipool.clone())
                    .collect::<Vec<_>>()
                    .as_slice(),
                &rpc,
                &interval_params,
            )
            .await;

        let factories = storage.inner.read().await.factories.clone();
        storage
            .run_factory_tasks(&factories, &rpc, monitoring_intgerval, &interval_params)
            .await;

        let sync_handle = spawn_syncing_task(
            ledger,
            storage.clone(),
            self.sync_interval.expect("Sync interval is not set"),
        );

        storage.append_handle(sync_handle).await;

        Ok(storage)
    }
}

pub fn spawn_syncing_task<L: Ledger + Send + 'static>(
    ledger: L,
    storage: MultipoolStorage,
    sync_interval: u64,
) -> JoinHandle<Result<()>> {
    tokio::task::spawn(async move {
        loop {
            let ir = storage.build_ir().await;
            ledger.write(ir)?.await?;
            tokio::time::sleep(Duration::from_millis(sync_interval)).await;
        }
    })
}
