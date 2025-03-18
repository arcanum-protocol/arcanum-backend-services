use std::{sync::Arc, time::Duration};

use alloy::network::EthereumWallet;
use alloy::primitives::address;
use alloy::signers::local::PrivateKeySigner;
use alloy::{primitives::Address, providers::ProviderBuilder};
use multipool_storage::{kafka::into_fetching_task, storage::MultipoolStorage};
use multipool_trader::{clickhouse::Click, TraderHook};
use multipool_types::kafka::KafkaTopics;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig,
};
use reqwest::Url;
use tokio::runtime::Handle;

const FACTORY_ADDRESS: Address = address!("1A9071F29731088650DbbB21a7bD7248a91d33cA");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let http_url = std::env::var("HTTP_URL").expect("HTTP_URL must be set");
    let pk_file = std::env::var("PRIVATE_KEY_FILE").expect("PRIVATE_KEY must be set");
    let group = std::env::var("KAFKA_GROUP").expect("KAFKA_GROUP must be set");
    let kafka_url = std::env::var("KAFKA_URL").expect("KAFKA_URL must be set");

    let pk = std::fs::read_to_string(pk_file).expect("Should have been able to read the file");
    let signer: PrivateKeySigner = pk.parse().expect("should parse private key");

    let wallet = EthereumWallet::from(signer);

    let th = TraderHook {
        click: Arc::new(Click::new().unwrap()),
        task_timeout: Duration::from_secs(2),
        handle: Handle::current(),
        rpc: ProviderBuilder::new()
            .wallet(wallet)
            .on_http(Url::parse(&http_url).unwrap()),
    };
    let db = sled::open("sled_db").unwrap();
    let mut storage = MultipoolStorage::init(db, th, FACTORY_ADDRESS)
        .await
        .unwrap();
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &group)
        .set("bootstrap.servers", &kafka_url)
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Creation failed");
    consumer
        .subscribe(&[
            KafkaTopics::ChainEvents.as_ref(),
            KafkaTopics::MpPrices.as_ref(),
        ])
        .expect("Failed to subscribe to topic");

    // into_fetching_task(&mut storage, pool, Duration::from_secs(1)).await?;

    into_fetching_task(&mut storage, consumer).await?;

    Ok(())
}
