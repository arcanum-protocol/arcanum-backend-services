use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, IntoLogData},
    providers::Provider,
    rpc::types::{Filter, Log},
    sol_types::{SolEvent, SolEventInterface},
};
use dashmap::DashSet;
use futures::{future::try_join_all, StreamExt};
use multipool::Multipool;
use tokio::{
    sync::{mpsc, watch, RwLock},
    task::JoinHandle,
    time::{self},
};
use tokio_stream::wrappers::{IntervalStream, WatchStream};

use crate::{
    contracts::{
        Multipool::{AssetChange, MultipoolEvents, TargetShareChange},
        MultipoolFactory::{MultipoolFactoryInstance, MultipoolSpawned},
    },
    raw_storage::RawEventStorage,
};

#[derive(Clone)]
pub struct Scheduler {
    pub new_multipool_fetch_tick_interval_millis: u64,
    pub multipool_events_fetch_tick_interval_millis: u64,
}

impl Scheduler {
    pub fn new(
        new_multipool_fetch_tick_interval_millis: u64,
        multipool_events_fetch_tick_interval_millis: u64,
    ) -> Self {
        Self {
            new_multipool_fetch_tick_interval_millis,
            multipool_events_fetch_tick_interval_millis,
        }
    }

    pub fn run(
        &self,
        new_multipool_fetch_tick_sender: watch::Sender<()>,
        multipool_events_tick_sender: watch::Sender<()>,
    ) {
        let mut new_multipool_fetch_tick_interval_stream = IntervalStream::new(time::interval(
            Duration::from_millis(self.new_multipool_fetch_tick_interval_millis),
        ));
        let mut multipool_events_ticker_interval_stream = IntervalStream::new(time::interval(
            Duration::from_millis(self.multipool_events_fetch_tick_interval_millis),
        ));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(_) = new_multipool_fetch_tick_interval_stream.next() => {
                        if let Err(_) = new_multipool_fetch_tick_sender.send(()) {
                            break;
                        }
                    }
                    Some(_) = multipool_events_ticker_interval_stream.next() => {
                        if let Err(_) = multipool_events_tick_sender.send(()) {
                            break;
                        }
                    }
                    else => {
                        break;
                    }
                }
            }
        });
    }
}

#[derive(Clone)]
pub struct IntervalConfig {
    pub new_multipool_fetch_tick_interval_millis: u64,
    pub multipool_events_ticker_interval_millis: u64,
}

pub struct MultipoolIndexer<P, R: RawEventStorage> {
    factory_contract_address: Address,
    chain_id: String,
    last_observed_block: AtomicU64,
    raw_storage: R,
    provider: P,
    ws_provider: Option<P>,
    multipool_storage: crate::multipool_storage::MultipoolStorage,
    scheduler: Scheduler,
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
        intervals: IntervalConfig,
        enable_ws: bool,
    ) -> anyhow::Result<Self> {
        let ticker = Scheduler::new(
            intervals.new_multipool_fetch_tick_interval_millis,
            intervals.multipool_events_ticker_interval_millis,
        );

        Ok(Self {
            factory_contract_address: factory_address,
            chain_id: provider.get_chain_id().await?.to_string(),
            last_observed_block: AtomicU64::new(from_block),
            raw_storage,
            provider,
            ws_provider,
            multipool_storage,
            scheduler: ticker,
            enable_ws,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (new_multipool_fetch_tick_sender, new_multipool_fetch_tick_receiver) =
            watch::channel(());

        let (multipool_events_fetch_tick_sender, multipool_events_tick_receiver) =
            watch::channel(());

        self.scheduler.run(
            new_multipool_fetch_tick_sender.clone(),
            multipool_events_fetch_tick_sender.clone(),
        );

        if self.enable_ws {
            self.spawn_ws_watcher(new_multipool_fetch_tick_sender)
                .await?;
        }
        self.spawn_interval_polling_task(
            new_multipool_fetch_tick_receiver,
            multipool_events_tick_receiver,
        )
        .await?;

        Ok(())
    }

    pub async fn spawn_ws_watcher(
        &self,
        new_multipool_fetch_tick_sender: watch::Sender<()>,
    ) -> anyhow::Result<()> {
        let ws_provider = self
            .ws_provider
            .clone()
            .ok_or(anyhow::anyhow!("WS provider not provided"))?;
        let factory_instance =
            MultipoolFactoryInstance::new(self.factory_contract_address, ws_provider);

        let mut new_multipool_event_filter = factory_instance
            .MultipoolSpawned_filter()
            .from_block(BlockNumberOrTag::Number(
                self.last_observed_block.load(Ordering::Relaxed),
            ))
            .subscribe()
            .await?
            .into_stream();

        tokio::spawn(async move {
            loop {
                if let Some(Ok((_, _))) = new_multipool_event_filter.next().await {
                    if let Err(_) = new_multipool_fetch_tick_sender.send(()) {
                        break;
                    }
                }
            }
        });

        // TODO add multipool watching

        Ok(())
    }

    pub async fn spawn_interval_polling_task(
        self,
        new_multipool_fetch_tick_receiver: watch::Receiver<()>,
        multipool_events_tick_receiver: watch::Receiver<()>,
    ) -> anyhow::Result<()> {
        let mut new_mp_fetch_tick_receiver = WatchStream::new(new_multipool_fetch_tick_receiver);
        let mut multipool_events_tick_receiver = WatchStream::new(multipool_events_tick_receiver);

        loop {
            tokio::select! {
                Some(_) = new_mp_fetch_tick_receiver.next() => {
                    self.handle_new_multipools_fetch_tick().await?;
                }
                Some(_) = multipool_events_tick_receiver.next() => {
                    self.handle_multipool_events_tick().await?;
                }
                else => break,
            }
        }

        Ok(())
    }

    async fn handle_new_multipools_fetch_tick(&self) -> anyhow::Result<()> {
        let (new_multipools, last_block_number) = fetch_new_multipools(
            self.factory_contract_address.clone(),
            self.last_observed_block.load(Ordering::Relaxed),
            self.provider.clone(),
        )
        .await?;
        for (mp_spawned_event, log) in new_multipools {
            let mp_address = mp_spawned_event._0.clone();
            self.raw_storage
                .insert_event(
                    &self.factory_contract_address.to_string(),
                    &self.chain_id,
                    log.block_number.unwrap().try_into().unwrap(),
                    mp_spawned_event,
                )
                .await?;
            self.multipool_storage
                .insert_multipool(mp_address, log.block_number.unwrap().try_into().unwrap())?;
        }
        self.last_observed_block
            .store(last_block_number, Ordering::Relaxed);
        self.update_last_block_number(last_block_number).await?;
        Ok(())
    }

    async fn update_last_block_number(&self, block_number: u64) -> anyhow::Result<()> {
        self.raw_storage
            .update_last_observed_block_number(&self.chain_id, block_number.try_into().unwrap())
            .await?;
        Ok(())
    }

    // TODO apply events to multipool structs
    async fn handle_multipool_events_tick(&self) -> anyhow::Result<()> {
        let provider = self.provider.clone();
        let raw_storage = self.raw_storage.clone();
        let chain_id = self.chain_id.clone();

        let last_observed_block_number = self.last_observed_block.load(Ordering::Relaxed);
        let (mut updates, last_block_number) =
            fetch_multipool_events(last_observed_block_number, provider.clone()).await?;

        self.filter_multipool_events(&mut updates);

        for (event, log) in updates.asset_changes {
            raw_storage
                .insert_event(
                    &log.inner.address.to_string(),
                    &provider.get_chain_id().await?.to_string(),
                    log.block_number.unwrap().try_into().unwrap(),
                    event,
                )
                .await?;
        }

        for (event, log) in updates.target_share_changes {
            raw_storage
                .insert_event(
                    &log.inner.address.to_string(),
                    &provider.get_chain_id().await?.to_string(),
                    log.block_number.unwrap().try_into().unwrap(),
                    event,
                )
                .await?;
        }

        self.last_observed_block
            .store(last_block_number, Ordering::Relaxed);
        raw_storage
            .update_last_observed_block_number(&chain_id, last_block_number.try_into().unwrap())
            .await?;

        Ok(())
    }

    pub fn filter_multipool_events(&self, batch: &mut NewMultipoolEventsBatch) {
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

async fn fetch_new_multipools<P: Provider>(
    factory_address: Address,
    from_block: u64,
    provider: P,
) -> anyhow::Result<(Vec<(MultipoolSpawned, Log)>, u64)> {
    let last_block_number = provider.get_block_number().await?;
    let logs = MultipoolFactoryInstance::new(factory_address, provider)
        .MultipoolSpawned_filter()
        .from_block(from_block)
        .to_block(last_block_number - 1)
        .query()
        .await?;

    Ok((logs, last_block_number))
}

#[derive(Default)]
struct NewMultipoolEventsBatch {
    asset_changes: Vec<(AssetChange, Log)>,
    target_share_changes: Vec<(TargetShareChange, Log)>,
}

async fn fetch_multipool_events<P: Provider>(
    from_block: u64,
    provider: P,
) -> anyhow::Result<(NewMultipoolEventsBatch, u64)> {
    let last_block_number = provider.get_block_number().await?;
    let filter = Filter::new()
        .events(vec![AssetChange::SIGNATURE])
        .from_block(from_block)
        .to_block(last_block_number - 1);

    let logs = provider.get_logs(&filter).await?;

    let mut batch = NewMultipoolEventsBatch::default();

    for log in logs.iter() {
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
