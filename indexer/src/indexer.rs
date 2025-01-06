use std::{sync::Arc, time::Duration};

use alloy::{eips::BlockNumberOrTag, primitives::Address, providers::Provider, rpc::types::Log};
use dashmap::DashSet;
use futures::{future::try_join_all, StreamExt};
use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::{self},
};
use tokio_stream::wrappers::{IntervalStream, ReceiverStream};

use crate::{
    contracts::{
        AssetChangeEvent, Multipool::MultipoolInstance, MultipoolFactory::MultipoolFactoryInstance,
        MultipoolSpawnedEvent, TargetShareChangeEvent,
    },
    raw_storage::RawEventStorage,
};

#[derive(Clone)]
pub struct Ticker {
    pub new_multipool_fetch_tick_interval_millis: u64,
    pub multipool_events_fetch_tick_interval_millis: u64,
}

impl Ticker {
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
        new_multipool_fetch_tick_sender: mpsc::Sender<()>,
        multipool_events_tick_sender: mpsc::Sender<()>,
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
                        if let Err(_) = new_multipool_fetch_tick_sender.send(()).await {
                            break;
                        }
                    }
                    Some(_) = multipool_events_ticker_interval_stream.next() => {
                        if let Err(_) = multipool_events_tick_sender.send(()).await {
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
    last_observed_block: Arc<RwLock<BlockNumberOrTag>>,
    raw_storage: R,
    provider: P,
    multipool_storage: crate::multipool_storage::MultipoolStorage,
    ticker: Ticker,
    enable_ws: bool,
    watched_multipools: DashSet<Address>,
}

impl<P: Provider + Clone + 'static, R: RawEventStorage + Clone + Send + Sync + 'static>
    MultipoolIndexer<P, R>
{
    pub async fn new(
        factory_address: Address,
        provider: P,
        from_block: BlockNumberOrTag,
        raw_storage: R,
        multipool_storage: crate::multipool_storage::MultipoolStorage,
        intervals: IntervalConfig,
        enable_ws: bool,
    ) -> anyhow::Result<Self> {
        let ticker = Ticker::new(
            intervals.new_multipool_fetch_tick_interval_millis,
            intervals.multipool_events_ticker_interval_millis,
        );

        Ok(Self {
            factory_contract_address: factory_address,
            chain_id: provider.get_chain_id().await?.to_string(),
            last_observed_block: Arc::new(RwLock::new(from_block)),
            raw_storage,
            provider,
            multipool_storage,
            ticker,
            enable_ws,
            watched_multipools: DashSet::new(),
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (new_multipool_fetch_tick_sender, new_multipool_fetch_tick_receiver) = mpsc::channel(0);

        let (multipool_events_tick_sender, multipool_events_tick_receiver) = mpsc::channel(0);

        self.ticker.run(
            new_multipool_fetch_tick_sender.clone(),
            multipool_events_tick_sender.clone(),
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
        new_multipool_fetch_tick_sender: mpsc::Sender<()>,
    ) -> anyhow::Result<()> {
        let factory_instance =
            MultipoolFactoryInstance::new(self.factory_contract_address, self.provider.clone());

        let mut multipool_creation_filter = factory_instance
            .MultipoolSpawned_filter()
            .from_block(self.last_observed_block.read().await.clone())
            .subscribe()
            .await?
            .into_stream();

        tokio::spawn(async move {
            loop {
                if let Some(Ok((_, _))) = multipool_creation_filter.next().await {
                    if let Err(_) = new_multipool_fetch_tick_sender.send(()).await {
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
        new_multipool_fetch_tick_receiver: mpsc::Receiver<()>,
        multipool_events_tick_receiver: mpsc::Receiver<()>,
    ) -> anyhow::Result<()> {
        let mut new_mp_fetch_tick_receiver = ReceiverStream::new(new_multipool_fetch_tick_receiver);
        let mut multipool_events_tick_receiver =
            ReceiverStream::new(multipool_events_tick_receiver);

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
            self.last_observed_block.read().await.clone(),
            self.provider.clone(),
        )
        .await?;
        for (mp_spawned_event, log) in new_multipools {
            let mp_address = mp_spawned_event.address.clone();
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
            self.watched_multipools.insert(mp_address);
        }
        *self.last_observed_block.write().await =
            BlockNumberOrTag::Number(last_block_number.into());
        self.update_last_block_number(last_block_number).await?;
        Ok(())
    }

    async fn update_last_block_number(&self, block_number: u64) -> anyhow::Result<()> {
        self.raw_storage
            .update_last_observed_block_number(&self.chain_id, block_number.try_into().unwrap())
            .await?;
        Ok(())
    }

    async fn handle_multipool_events_tick(&self) -> anyhow::Result<()> {
        let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = vec![];

        for mp_address in self.watched_multipools.iter() {
            let mp_address = mp_address.clone();
            let provider = self.provider.clone();
            // TODO update multipool structs with new data
            // let multipool_storage = multipool_storage.clone();
            let raw_storage = self.raw_storage.clone();
            let chain_id = self.chain_id.clone();

            let last_block_number_mutex = self.last_observed_block.clone();

            tasks.push(tokio::spawn(async move {
                let last_observed_block_number = last_block_number_mutex.read().await.clone();
                let (asset_change_events, _) = fetch_asset_change_events(
                    mp_address,
                    last_observed_block_number,
                    provider.clone(),
                )
                .await?;

                let (target_share_change_events, last_block_number) =
                    fetch_target_share_change_events(
                        mp_address,
                        last_observed_block_number,
                        provider.clone(),
                    )
                    .await?;

                for (event, log) in asset_change_events {
                    raw_storage
                        .insert_event(
                            &mp_address.to_string(),
                            &provider.get_chain_id().await?.to_string(),
                            log.block_number.unwrap().try_into().unwrap(),
                            event,
                        )
                        .await?;
                }

                for (event, log) in target_share_change_events {
                    raw_storage
                        .insert_event(
                            &mp_address.to_string(),
                            &provider.get_chain_id().await?.to_string(),
                            log.block_number.unwrap().try_into().unwrap(),
                            event,
                        )
                        .await?;
                }
                *last_block_number_mutex.write().await =
                    BlockNumberOrTag::Number(last_block_number.into());
                raw_storage
                    .update_last_observed_block_number(
                        &chain_id,
                        last_block_number.try_into().unwrap(),
                    )
                    .await?;

                Ok(())
            }));
        }

        try_join_all(tasks).await?;

        Ok(())
    }
}

async fn fetch_new_multipools<P: Provider>(
    factory_address: Address,
    from_block: BlockNumberOrTag,
    provider: P,
) -> anyhow::Result<(Vec<(MultipoolSpawnedEvent, Log)>, u64)> {
    let last_block_number = provider.get_block_number().await?;
    let logs = MultipoolFactoryInstance::new(factory_address, provider)
        .MultipoolSpawned_filter()
        .from_block(from_block)
        .to_block(last_block_number - 1)
        .query()
        .await?;

    Ok((
        logs.into_iter()
            .map(|(event, log)| (MultipoolSpawnedEvent::new_from_event(event), log))
            .collect(),
        last_block_number,
    ))
}

async fn fetch_asset_change_events<P: Provider>(
    multipool_address: Address,
    from_block: BlockNumberOrTag,
    provider: P,
) -> anyhow::Result<(Vec<(AssetChangeEvent, Log)>, u64)> {
    let last_block_number = provider.get_block_number().await?;
    let logs = MultipoolInstance::new(multipool_address, provider)
        .AssetChange_filter()
        .from_block(from_block)
        .to_block(last_block_number - 1)
        .query()
        .await?;

    Ok((
        logs.into_iter()
            .map(|(event, log)| (AssetChangeEvent::new_from_event(event), log))
            .collect(),
        last_block_number,
    ))
}

async fn fetch_target_share_change_events<P: Provider>(
    multipool_address: Address,
    from_block: BlockNumberOrTag,
    provider: P,
) -> anyhow::Result<(Vec<(TargetShareChangeEvent, Log)>, u64)> {
    let last_block_number = provider.get_block_number().await?;
    let logs = MultipoolInstance::new(multipool_address, provider)
        .TargetShareChange_filter()
        .from_block(from_block)
        .to_block(last_block_number - 1)
        .query()
        .await?;

    Ok((
        logs.into_iter()
            .map(|(event, log)| (TargetShareChangeEvent::new_from_event(event), log))
            .collect(),
        last_block_number,
    ))
}
