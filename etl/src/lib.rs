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
pub struct EtlService {
    rpc_url: String,
    kafka_url: String,
    kafka_group: String,
    chain_id: u64,
}

impl ServiceData for EtlService {
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

        loop {
            let mut stream = consumer.stream();
            // add better error handling
            while let Some(Ok(message)) = stream.next().await {
                match message.topic().try_into()? {
                    KafkaTopics::ChainEvents(_chain_id) => {
                        let bytes = message
                            .payload()
                            .context(anyhow!("Received message with no payload"))?;
                        let blocks = messages::Block::unpack(bytes);
                        storage.apply_events(vec![blocks].try_into()?).await?;
                    }
                    KafkaTopics::MpPrices(_chain_id) => {
                        let bytes = message
                            .payload()
                            .context(anyhow!("Received message with no payload"))?;
                        let data = PriceData::unpack(bytes);
                        storage.apply_prices(data.address, data.prices).await?;
                    }
                }
                consumer.commit_message(&message, CommitMode::Sync)?;
            }
        }

        Ok(())
    }
}
