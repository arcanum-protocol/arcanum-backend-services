use std::time::Duration;

use alloy::primitives::address;
use alloy::{primitives::Address, providers::ProviderBuilder};
use multipool_storage::{kafka::into_fetching_task, storage::MultipoolStorage};
use multipool_types::kafka::KafkaTopics;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::FutureProducer,
    ClientConfig,
};
use reqwest::Url;

use crate::processor::PriceFetcher;

mod processor;

const FACTORY_ADDRESS: Address = address!("1A9071F29731088650DbbB21a7bD7248a91d33cA");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let http_url = std::env::var("HTTP_URL").expect("HTTP_URL must be set");
    let group = std::env::var("KAFKA_GROUP").expect("KAFKA_GROUP must be set");
    let kafka_url = std::env::var("KAFKA_URL").expect("KAFKA_URL must be set");

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_url)
        .create()
        .expect("Cannot create kafka producer");

    let th = PriceFetcher {
        producer,
        delay: Duration::from_secs(2),
        multicall_chunk_size: 5,
        rpc: ProviderBuilder::new().on_http(Url::parse(&http_url).unwrap()),
    };
    let db = sled::open("sled_db").unwrap();
    let mut storage = MultipoolStorage::init(db, th, FACTORY_ADDRESS)
        .await
        .unwrap();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &group)
        .set("bootstrap.servers", &kafka_url)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Creation failed");

    consumer
        .subscribe(&[KafkaTopics::ChainEvents.as_ref()])
        .expect("Failed to subscribe to topic");

    into_fetching_task(&mut storage, consumer).await?;

    Ok(())
}
