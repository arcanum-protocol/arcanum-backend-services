use std::time::Duration;

use alloy::primitives::address;
use alloy::{primitives::Address, providers::ProviderBuilder};
use backend_service::ServiceData;
use multipool_storage::{kafka::into_fetching_task, storage::MultipoolStorage};
use multipool_types::messages::KafkaTopics;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::FutureProducer,
    ClientConfig,
};
use reqwest::Url;
use serde::Deserialize;

use crate::processor::PriceFetcher;

mod processor;

const FACTORY_ADDRESS: Address = address!("1A9071F29731088650DbbB21a7bD7248a91d33cA");

#[derive(Deserialize)]
pub struct PriceFetcherService {
    rpc_url: String,
    kafka_url: String,
    kafka_group: String,
    chain_id: u64,
}

impl ServiceData for PriceFetcherService {
    async fn run(self) -> anyhow::Result<()> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &self.kafka_url)
            .create()
            .expect("Cannot create kafka producer");

        let th = PriceFetcher {
            producer,
            delay: Duration::from_secs(2),
            chain_id: self.chain_id,
            multicall_chunk_size: 5,
            rpc: ProviderBuilder::new().on_http(Url::parse(&self.rpc_url).unwrap()),
        };
        let db = sled::open("sled_db").unwrap();
        let mut storage = MultipoolStorage::init(db, th, FACTORY_ADDRESS)
            .await
            .unwrap();

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &self.kafka_group)
            .set("bootstrap.servers", &self.kafka_url)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Creation failed");

        consumer
            .subscribe(&[KafkaTopics::ChainEvents(self.chain_id).to_string().as_str()])
            .expect("Failed to subscribe to topic");

        into_fetching_task(&mut storage, consumer).await?;

        Ok(())
    }
}
