use std::time::Duration;

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, U64},
    providers::Provider,
    rpc::types::Log,
};
use futures::StreamExt;
use tokio::{
    sync::mpsc,
    time::{self, Interval},
};
use tokio_stream::wrappers::{IntervalStream, ReceiverStream};

use crate::{
    contracts::{
        AssetChangeEvent,
        Multipool::{self, MultipoolInstance},
        MultipoolFactory::{self, MultipoolFactoryInstance},
        MultipoolSpawnedEvent, TargetShareChangeEvent,
    },
    raw_storage::RawEventStorage,
};

#[derive(Clone)]
pub struct Ticker {
    pub new_multipool_fetch_tick_sender: mpsc::Sender<()>,
    pub new_multipool_fetch_tick_interval_millis: u64,
    pub multipool_events_fetch_tick_sender: mpsc::Sender<()>,
    pub multipool_events_fetch_tick_interval_millis: u64,
}

impl Ticker {
    pub fn new(
        new_multipool_fetch_tick_sender: mpsc::Sender<()>,
        multipool_events_fetch_tick_sender: mpsc::Sender<()>,
        new_multipool_fetch_tick_interval_millis: u64,
        multipool_events_fetch_tick_interval_millis: u64,
    ) -> Self {
        Self {
            new_multipool_fetch_tick_sender,
            new_multipool_fetch_tick_interval_millis,
            multipool_events_fetch_tick_sender,
            multipool_events_fetch_tick_interval_millis,
        }
    }

    pub fn run(&self) {
        let multipool_creation_ticker = self.new_multipool_fetch_tick_sender.clone();
        let multipool_events_ticker = self.multipool_events_fetch_tick_sender.clone();

        let mut multipool_creation_ticker_interval_stream = IntervalStream::new(time::interval(
            Duration::from_millis(self.new_multipool_fetch_tick_interval_millis),
        ));
        let mut multipool_events_ticker_interval_stream = IntervalStream::new(time::interval(
            Duration::from_millis(self.multipool_events_fetch_tick_interval_millis),
        ));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(_) = multipool_creation_ticker_interval_stream.next() => {
                        if let Err(_) = multipool_creation_ticker.send(()).await {
                            break;
                        }
                    }
                    Some(_) = multipool_events_ticker_interval_stream.next() => {
                        if let Err(_) = multipool_events_ticker.send(()).await {
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
    contract_address: Address,
    factory_contract_address: Address,
    chain_id: String,
    from_block: BlockNumberOrTag,
    raw_storage: R,
    provider: P,
    multipool_storage: crate::multipool_storage::MultipoolStorage,
    ticker: Ticker,
    new_multipool_fetch_tick_receiver: mpsc::Receiver<()>,
    multipool_events_fetch_tick_receiver: mpsc::Receiver<()>,
}

impl<P: Provider + Clone + 'static, R: RawEventStorage + Clone + Send + Sync + 'static>
    MultipoolIndexer<P, R>
{
    pub async fn new(
        contract_address: Address,
        factory_address: Address,
        provider: P,
        from_block: BlockNumberOrTag,
        raw_storage: R,
        multipool_storage: crate::multipool_storage::MultipoolStorage,
        intervals: IntervalConfig,
    ) -> anyhow::Result<Self> {
        let (new_multipool_fetch_tick_sender, new_multipool_fetch_tick_receiver) = mpsc::channel(1);
        let (multipool_events_fetch_tick_sender, multipool_events_fetch_tick_receiver) =
            mpsc::channel(1);

        let ticker = Ticker::new(
            new_multipool_fetch_tick_sender,
            multipool_events_fetch_tick_sender,
            intervals.new_multipool_fetch_tick_interval_millis,
            intervals.multipool_events_ticker_interval_millis,
        );

        Ok(Self {
            contract_address,
            factory_contract_address: factory_address,
            chain_id: provider.get_chain_id().await?.to_string(),
            from_block,
            raw_storage,
            provider,
            multipool_storage,
            ticker,
            new_multipool_fetch_tick_receiver,
            multipool_events_fetch_tick_receiver,
        })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        self.ticker.run();
        self.spawn_event_detector().await?;
        self.spawn_polling_task().await?;

        Ok(())
    }

    pub async fn spawn_event_detector(&self) -> anyhow::Result<()> {
        let factory_instance =
            MultipoolFactoryInstance::new(self.factory_contract_address, self.provider.clone());
        let contract_instance =
            MultipoolInstance::new(self.contract_address, self.provider.clone());
        let from_block = self.from_block.clone();
        let ticker = self.ticker.clone();

        let mut multipool_creation_filter = factory_instance
            .MultipoolSpawned_filter()
            .from_block(from_block)
            .watch()
            .await?
            .into_stream();
        let mut asset_change_filter = contract_instance
            .AssetChange_filter()
            .from_block(from_block)
            .watch()
            .await?
            .into_stream();
        let mut target_share_change_filter = contract_instance
            .TargetShareChange_filter()
            .from_block(from_block)
            .watch()
            .await?
            .into_stream();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(Ok((event, log))) = multipool_creation_filter.next() => {
                        if let Err(_) = ticker.new_multipool_fetch_tick_sender.send(()).await {
                            break;
                        }
                    }
                    Some(Ok((event, log))) = asset_change_filter.next() => {
                        if let Err(_) = ticker.multipool_events_fetch_tick_sender.send(()).await {
                            break;
                        }
                    }
                    Some(Ok((event, log))) = target_share_change_filter.next() => {
                        if let Err(_) = ticker.multipool_events_fetch_tick_sender.send(()).await {
                            break;
                        }
                    }
                    else => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn spawn_polling_task(mut self) -> anyhow::Result<()> {
        let mut new_mp_fetch_tick_receiver =
            ReceiverStream::new(self.new_multipool_fetch_tick_receiver);
        let mut mp_events_fetch_tick_receiver =
            ReceiverStream::new(self.multipool_events_fetch_tick_receiver);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(_) = new_mp_fetch_tick_receiver.next() => {
                        let (new_multipools, last_block_number) = fetch_new_multipools(
                            self.factory_contract_address.clone(),
                            self.from_block.clone(),
                            self.provider.clone(),
                        ).await.unwrap();
                        self.from_block = BlockNumberOrTag::Number(last_block_number.into());
                        for (mp_spawned_event, log) in new_multipools {
                            let mp_address = mp_spawned_event.address.clone();
                            self.raw_storage
                                .insert_event(
                                    &self.factory_contract_address.to_string(),
                                    &self.chain_id,
                                    log.block_number.unwrap().try_into().unwrap(),
                                    mp_spawned_event,
                                )
                                .await
                                .unwrap();
                            self.multipool_storage
                                .insert_multipool(
                                    mp_address,
                                    log.block_number
                                        .unwrap()
                                        .try_into()
                                        .unwrap())
                                .unwrap();
                        }
                    }
                    Some(_) = mp_events_fetch_tick_receiver.next() => {
                        let (asset_change_events, _) = fetch_asset_change_events(
                            self.contract_address.clone(),
                            self.from_block.clone(),
                            self.provider.clone(),
                        )
                        .await
                        .unwrap();
                        for (asset_change_event, log) in asset_change_events {
                            self.raw_storage
                                .insert_event(
                                    &self.contract_address.to_string(),
                                    &self.chain_id,
                                    log.block_number.unwrap().try_into().unwrap(),
                                    asset_change_event,
                                )
                                .await
                                .unwrap();
                        }

                        let (target_share_change_events, last_block_number) = fetch_target_share_change_events(
                            self.contract_address.clone(),
                            self.from_block.clone(),
                            self.provider.clone(),
                        )
                        .await
                        .unwrap();
                        for (target_share_change_event, log) in target_share_change_events {
                            self.raw_storage
                                .insert_event(
                                    &self.contract_address.to_string(),
                                    &self.chain_id,
                                    log.block_number.unwrap().try_into().unwrap(),
                                    target_share_change_event,
                                )
                                .await
                                .unwrap();
                        }
                        self.from_block = BlockNumberOrTag::Number(last_block_number.into());
                    }
                    else => {
                        break;
                    }
                }
            }
        });

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
    contract_address: Address,
    from_block: BlockNumberOrTag,
    provider: P,
) -> anyhow::Result<(Vec<(AssetChangeEvent, Log)>, u64)> {
    let last_block_number = provider.get_block_number().await?;
    let logs = MultipoolInstance::new(contract_address, provider)
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
