use std::time::Duration;

use alloy::{
    primitives::Address,
    providers::Provider,
    rpc::types::{Filter, Log},
    sol_types::{SolEvent, SolEventInterface},
};
use tokio::{sync::watch, time::interval};
use tokio_stream::{wrappers::WatchStream, StreamExt as StreamExtTokio};

use crate::{
    contracts::{
        Multipool::{AssetChange, MultipoolEvents, TargetShareChange},
        MultipoolFactory::{MultipoolFactoryEvents, MultipoolSpawned},
    },
    raw_storage::RawEventStorage,
};

pub struct MultipoolIndexer<P, R: RawEventStorage> {
    factory_contract_address: Address,
    chain_id: String,
    last_observed_block: u64,
    raw_storage: R,
    provider: P,
    ws_provider: Option<P>,
    multipool_storage: crate::multipool_storage::MultipoolStorage,
    poll_interval_millis: u64,
    enable_ws: bool,
}

impl<P: Provider + Clone + 'static, R: RawEventStorage + Clone + Send + Sync + 'static>
    MultipoolIndexer<P, R>
{
    pub async fn new(
        factory_address: Address,
        provider: P,
        ws_provider: Option<P>,
        from_block: u64,
        raw_storage: R,
        multipool_storage: crate::multipool_storage::MultipoolStorage,
        enable_ws: bool,
        poll_interval_millis: u64,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            factory_contract_address: factory_address,
            chain_id: provider.get_chain_id().await?.to_string(),
            last_observed_block: from_block,
            raw_storage,
            provider,
            ws_provider,
            multipool_storage,
            poll_interval_millis,
            enable_ws,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (fetch_tick_sender, fetch_tick_receiver) = watch::channel(());

        tokio::spawn({
            let mut poll_interval = interval(Duration::from_millis(self.poll_interval_millis));
            let fetch_tick_sender = fetch_tick_sender.clone();
            async move {
                loop {
                    poll_interval.tick().await;
                    if let Err(_) = fetch_tick_sender.send(()) {
                        break;
                    }
                }
            }
        });

        if self.enable_ws {
            self.spawn_ws_watcher(fetch_tick_sender).await?;
        }
        self.spawn_interval_polling_task(fetch_tick_receiver)
            .await?;

        Ok(())
    }

    pub async fn spawn_ws_watcher(
        &self,
        new_event_notifier: watch::Sender<()>,
    ) -> anyhow::Result<()> {
        let ws_provider = self
            .ws_provider
            .clone()
            .ok_or(anyhow::anyhow!("WS provider not provided"))?;

        let new_multipool_event_filter = build_multipool_event_filter(self.last_observed_block);
        let mut subscription = ws_provider
            .subscribe_logs(&new_multipool_event_filter)
            .await?;

        tokio::spawn(async move {
            loop {
                if let Ok(_) = subscription.recv().await {
                    if let Err(_) = new_event_notifier.send(()) {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn spawn_interval_polling_task(
        mut self,
        fetch_tick_recv: watch::Receiver<()>,
    ) -> anyhow::Result<()> {
        let mut fetch_tick_recv = WatchStream::new(fetch_tick_recv);

        loop {
            if let None = fetch_tick_recv.next().await {
                break;
            }

            self.handle_multipool_events_tick().await?;
        }

        Ok(())
    }

    async fn update_last_block_number(&self, block_number: u64) -> anyhow::Result<()> {
        self.raw_storage
            .update_last_observed_block_number(&self.chain_id, block_number)
            .await?;
        Ok(())
    }

    // TODO apply events to multipool structs
    async fn handle_multipool_events_tick(&mut self) -> anyhow::Result<()> {
        let (mut batch, last_block_number) =
            fetch_multipool_events(self.last_observed_block, self.provider.clone()).await?;

        self.filter_multipool_events(&mut batch);

        for (mp_spawned_event, log) in batch.multipools_spawned {
            let mp_address = mp_spawned_event._0.clone();
            self.raw_storage
                .insert_event(
                    &self.factory_contract_address.to_string(),
                    &self.chain_id,
                    log.block_number
                        .ok_or(anyhow::anyhow!("no block number in response"))?,
                    log.block_timestamp
                        .ok_or(anyhow::anyhow!("no block timestamp in response"))?,
                    mp_spawned_event,
                )
                .await?;
            self.multipool_storage
                .insert_multipool(mp_address, log.block_number.unwrap().try_into().unwrap())?;
        }

        for (event, log) in batch.asset_changes {
            self.raw_storage
                .insert_event(
                    &log.inner.address.to_string(),
                    &self.provider.get_chain_id().await?.to_string(),
                    log.block_number
                        .ok_or(anyhow::anyhow!("no block number in response"))?,
                    log.block_timestamp
                        .ok_or(anyhow::anyhow!("no block timestamp in response"))?,
                    event,
                )
                .await?;
        }

        for (event, log) in batch.target_share_changes {
            self.raw_storage
                .insert_event(
                    &log.inner.address.to_string(),
                    &self.provider.get_chain_id().await?.to_string(),
                    log.block_number
                        .ok_or(anyhow::anyhow!("no block number in response"))?,
                    log.block_timestamp
                        .ok_or(anyhow::anyhow!("no block timestamp in response"))?,
                    event,
                )
                .await?;
        }

        self.last_observed_block = last_block_number;
        self.update_last_block_number(last_block_number).await?;

        Ok(())
    }

    pub fn filter_multipool_events(&self, batch: &mut MultipoolEventsBatch) {
        batch
            .multipools_spawned
            .retain(|(_, log)| self.factory_contract_address == log.inner.address);

        batch.asset_changes.retain(|(_, log)| {
            self.multipool_storage
                .exists(log.inner.address)
                .unwrap_or(false)
        });

        batch.target_share_changes.retain(|(_, log)| {
            self.multipool_storage
                .exists(log.inner.address)
                .unwrap_or(false)
        });
    }
}

#[derive(Default)]
struct MultipoolEventsBatch {
    pub asset_changes: Vec<(AssetChange, Log)>,
    pub target_share_changes: Vec<(TargetShareChange, Log)>,
    pub multipools_spawned: Vec<(MultipoolSpawned, Log)>,
}

async fn fetch_multipool_events<P: Provider>(
    from_block: u64,
    provider: P,
) -> anyhow::Result<(MultipoolEventsBatch, u64)> {
    let last_block_number = provider.get_block_number().await?;
    let filter = build_multipool_event_filter(from_block).to_block(last_block_number - 1);

    let logs = provider.get_logs(&filter).await?;

    let mut batch = MultipoolEventsBatch::default();

    for log in logs.iter() {
        match MultipoolFactoryEvents::decode_log(&log.inner, true) {
            Ok(decoded_log) => match decoded_log.data {
                MultipoolFactoryEvents::MultipoolSpawned(multipool_spawned) => batch
                    .multipools_spawned
                    .push((multipool_spawned, log.clone())),
                _ => continue,
            },
            Err(_) => {}
        }
        let decoded_log = match MultipoolEvents::decode_log(&log.inner, true) {
            Ok(log) => log,
            Err(_) => continue,
        };

        match decoded_log.data {
            MultipoolEvents::AssetChange(asset_change) => {
                batch.asset_changes.push((asset_change, log.clone()))
            }
            MultipoolEvents::TargetShareChange(target_share_change) => batch
                .target_share_changes
                .push((target_share_change, log.clone())),

            _ => {}
        };
    }

    Ok((batch, last_block_number))
}

pub fn build_multipool_event_filter(from_block: u64) -> Filter {
    Filter::new()
        .events(vec![
            AssetChange::SIGNATURE,
            TargetShareChange::SIGNATURE,
            MultipoolSpawned::SIGNATURE,
        ])
        .from_block(from_block)
}
