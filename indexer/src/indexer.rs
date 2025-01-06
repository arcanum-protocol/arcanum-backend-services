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

    pub fn run(&self, new_multipool_fetch_tick_sender: mpsc::Sender<()>) {
        let mut multipool_creation_ticker_interval_stream = IntervalStream::new(time::interval(
            Duration::from_millis(self.new_multipool_fetch_tick_interval_millis),
        ));
        let mut multipool_events_ticker_interval_stream = IntervalStream::new(time::interval(
            Duration::from_millis(self.multipool_events_fetch_tick_interval_millis),
        ));

        tokio::spawn(async move {
            loop {
                if let None = multipool_creation_ticker_interval_stream.next().await {
                    break;
                }

                if let Err(_) = new_multipool_fetch_tick_sender.send(()).await {
                    break;
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
    from_block: BlockNumberOrTag,
    raw_storage: R,
    provider: P,
    multipool_storage: crate::multipool_storage::MultipoolStorage,
    ticker: Ticker,
    enable_ws: bool,
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
            from_block,
            raw_storage,
            provider,
            multipool_storage,
            ticker,
            enable_ws,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (new_multipool_fetch_tick_sender, new_multipool_fetch_tick_receiver) = mpsc::channel(1);

        self.ticker.run(new_multipool_fetch_tick_sender.clone());
        if self.enable_ws {
            self.spawn_event_detector(new_multipool_fetch_tick_sender)
                .await?;
        }
        self.spawn_polling_task(new_multipool_fetch_tick_receiver)
            .await;

        Ok(())
    }

    pub async fn spawn_event_detector(
        &self,
        new_multipool_fetch_tick_sender: mpsc::Sender<()>,
    ) -> anyhow::Result<()> {
        let factory_instance =
            MultipoolFactoryInstance::new(self.factory_contract_address, self.provider.clone());
        // let contract_instance =
        //     MultipoolInstance::new(self.contract_address, self.provider.clone());
        let from_block = self.from_block.clone();

        let mut multipool_creation_filter = factory_instance
            .MultipoolSpawned_filter()
            .from_block(from_block)
            .subscribe()
            .await?
            .into_stream();

        tokio::spawn(async move {
            loop {
                if let Some(Ok((_, _))) = multipool_creation_filter.next().await {
                    new_multipool_fetch_tick_sender.send(()).await.unwrap();
                }
            }
        });

        Ok(())
    }

    pub async fn spawn_polling_task(
        mut self,
        new_multipool_fetch_tick_receiver: mpsc::Receiver<()>,
    ) {
        let mut new_mp_fetch_tick_receiver = ReceiverStream::new(new_multipool_fetch_tick_receiver);

        loop {
            if let None = new_mp_fetch_tick_receiver.next().await {
                break;
            }
            let (new_multipools, last_block_number) = fetch_new_multipools(
                self.factory_contract_address.clone(),
                self.from_block.clone(),
                self.provider.clone(),
            )
            .await
            .unwrap();
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
                    .insert_multipool(mp_address, log.block_number.unwrap().try_into().unwrap())
                    .unwrap();
            }
        }
    }

    async fn update_last_block_number(&self, block_number: u64) -> anyhow::Result<()> {
        self.raw_storage
            .update_last_observed_block_number(&self.chain_id, block_number.try_into().unwrap())
            .await?;
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
