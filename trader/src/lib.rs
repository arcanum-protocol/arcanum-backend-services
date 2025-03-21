use backend_service::ServiceData;
use serde::Deserialize;
use std::{sync::Arc, time::Duration};

use crate::{
    clickhouse::{Click, ClickhouseConfig},
    hook::TraderHook,
};
use alloy::network::EthereumWallet;
use alloy::primitives::address;
use alloy::signers::local::PrivateKeySigner;
use alloy::{primitives::Address, providers::ProviderBuilder};
use multipool_storage::{kafka::into_fetching_task, storage::MultipoolStorage};
use multipool_types::messages::KafkaTopics;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig,
};
use reqwest::Url;
use tokio::runtime::Handle;

pub mod cashback;
pub mod clickhouse;
pub mod contracts;
pub mod execution;
pub mod hook;
pub mod strategies;
pub mod trade;
pub mod uniswap;

const FACTORY_ADDRESS: Address = address!("1A9071F29731088650DbbB21a7bD7248a91d33cA");

#[derive(Deserialize)]
pub struct TraderService {
    rpc_url: String,
    chain_id: u64,
    kafka_url: String,
    pk_file: String,
    kafka_group: String,
    clickhouse: ClickhouseConfig,
}

impl ServiceData for TraderService {
    async fn run(self) -> anyhow::Result<()> {
        let pk =
            std::fs::read_to_string(self.pk_file).expect("Should have been able to read the file");
        let signer: PrivateKeySigner = pk.parse().expect("should parse private key");

        let wallet = EthereumWallet::from(signer);

        let th = TraderHook {
            click: Arc::new(Click::new(self.clickhouse).unwrap()),
            task_timeout: Duration::from_secs(2),
            handle: Handle::current(),
            rpc: ProviderBuilder::new()
                .wallet(wallet)
                .on_http(Url::parse(&self.rpc_url).unwrap()),
        };
        let db = sled::open("sled_db").unwrap();
        let mut storage = MultipoolStorage::init(db, th, FACTORY_ADDRESS)
            .await
            .unwrap();
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &self.kafka_group)
            .set("bootstrap.servers", &self.kafka_url)
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Creation failed");
        consumer
            .subscribe(&[
                &KafkaTopics::ChainEvents(self.chain_id).to_string(),
                &KafkaTopics::MpPrices(self.chain_id).to_string(),
            ])
            .expect("Failed to subscribe to topic");

        // into_fetching_task(&mut storage, pool, Duration::from_secs(1)).await?;

        into_fetching_task(&mut storage, consumer).await?;

        Ok(())
    }
}
