use std::time::Duration;

use alloy::{
    primitives::Address,
    providers::{Provider, RootProvider},
    pubsub::PubSubFrontend,
    rpc::types::{Filter, Log},
    sol_types::{SolEvent, SolEventInterface},
    transports::http::{Client, Http},
};
use futures::{
    stream::{self},
    Stream,
};
use tokio::time::interval;
use tokio_stream::{wrappers::IntervalStream, StreamExt as StreamExtTokio};

use crate::{
    contracts::{
        Multipool::{AssetChange, MultipoolEvents, TargetShareChange},
        MultipoolFactory::MultipoolSpawned,
    },
    raw_storage::RawEventStorage,
};

pub struct MultipoolIndexer<R: RawEventStorage> {
    factory_contract_address: Address,
    chain_id: String,
    last_observed_block: u64,
    raw_storage: R,
    provider: RootProvider<Http<Client>>,
    ws_provider: Option<RootProvider<PubSubFrontend>>,
    multipool_storage: crate::multipool_storage::MultipoolStorage,
    poll_interval_millis: u64,
}

impl<R: RawEventStorage + Send + Sync + 'static> MultipoolIndexer<R> {
    pub async fn new(
        factory_address: Address,
        provider: RootProvider<Http<Client>>,
        ws_provider: Option<RootProvider<PubSubFrontend>>,
        from_block: u64,
        raw_storage: R,
        multipool_storage: crate::multipool_storage::MultipoolStorage,
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
        })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let poll_interval =
            IntervalStream::new(interval(Duration::from_millis(self.poll_interval_millis)));

        let ws_watcher = self.spawn_ws_watcher().await?;
        tokio::pin!(ws_watcher);
        tokio::pin!(poll_interval);

        loop {
            tokio::select! {
                Some(_) = ws_watcher.next() => {}
                Some(_) = poll_interval.next() => {}
                else => break,
            }

            self.handle_tick().await?;
        }

        Ok(())
    }

    pub async fn spawn_ws_watcher(&self) -> anyhow::Result<Box<dyn Stream<Item = Log> + Unpin>> {
        let ws_provider = match self.ws_provider {
            Some(ref ws_provider) => ws_provider.clone(),
            None => return Ok(Box::new(stream::empty())),
        };

        let new_multipool_event_filter = build_multipool_event_filter(self.last_observed_block);
        let subscription = ws_provider
            .subscribe_logs(&new_multipool_event_filter)
            .await?;
        Ok(Box::new(subscription.into_stream()))
    }

    async fn update_last_block_number(&self, block_number: u64) -> anyhow::Result<()> {
        self.raw_storage
            .update_last_observed_block_number(&self.chain_id, block_number)
            .await?;
        Ok(())
    }

    async fn handle_tick(&mut self) -> anyhow::Result<()> {
        let (mut events, last_block_number) = self.fetch_multipool_events().await?;

        // TODO derive multipool address and check event when unknown address emitted it
        // filter out events from contracts that were not created by the factory

        for (event, log) in events {
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
            // TODO apply events to multipool structs
        }
        self.last_observed_block = last_block_number;
        self.update_last_block_number(last_block_number).await?;

        Ok(())
    }

    async fn fetch_multipool_events(&self) -> anyhow::Result<(Vec<(MultipoolEvents, Log)>, u64)> {
        let last_block_number = self.provider.get_block_number().await?;
        let filter =
            build_multipool_event_filter(self.last_observed_block).to_block(last_block_number - 1);

        let logs = self.provider.get_logs(&filter).await?;

        let mut events = vec![];

        for log in logs.iter() {
            let decoded_log = match MultipoolEvents::decode_log(&log.inner, true) {
                Ok(log) => log,
                Err(_) => continue,
            };

            events.push((decoded_log.data, log.clone()));
        }

        Ok((events, last_block_number))
    }
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
