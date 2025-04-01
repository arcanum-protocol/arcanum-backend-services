use std::time::Duration;

use alloy::hex::ToHexExt;
use alloy::primitives::address;
use alloy::sol_types::SolEventInterface;
use alloy::{primitives::Address, providers::ProviderBuilder};
use backend_service::ServiceData;
use multipool_storage::storage::{parse_log, MultipoolStorage};
use multipool_types::Multipool::MultipoolEvents;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::FutureProducer,
    ClientConfig,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use anyhow::{anyhow, Context};
use futures::StreamExt;
use multipool_types::messages::{self, KafkaTopics, MsgPack, PriceData};
use rdkafka::consumer::CommitMode;
use rdkafka::Message;

use sqlx::PgPool;

use crate::processor::Etl;

mod processor;

const FACTORY_ADDRESS: Address = address!("1A9071F29731088650DbbB21a7bD7248a91d33cA");

#[derive(Deserialize)]
pub struct EtlService {
    rpc_url: String,
    kafka_url: String,
    kafka_group: String,
    chain_id: u64,
    database_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct TradingAction {
    account: String,
    multipool: String,
    chain_id: i64,
    action_type: String,
    quanitty: String,
    quote_quanitty: Option<String>,
    transaction_hash: String,
    timestamp: i64,
}

fn pg_bytes(bytes: &[u8]) -> String {
    format!("\\x{}", bytes.encode_hex())
}

impl ServiceData for EtlService {
    async fn run(self) -> anyhow::Result<()> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &self.kafka_url)
            .create()
            .expect("Cannot create kafka producer");

        let pool = PgPool::connect(&self.database_url).await?;

        let th = Etl {
            producer,
            delay: Duration::from_secs(2),
            chain_id: self.chain_id,
            pool: pool.clone(),
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
                        let block = messages::Block::unpack(bytes);

                        storage
                            .create_multipools(vec![block.clone()].as_slice().try_into()?)
                            .await?;

                        storage
                            .apply_events(vec![block.clone()].try_into()?)
                            .await?;

                        let actions: Vec<TradingAction> = block
                            .transactions
                            .iter()
                            .map(|txn| {
                                txn.events.iter().map(|event| {
                                    let parsed_log = MultipoolEvents::decode_log(&event.log, false)
                                        .unwrap()
                                        .data;
                                    let mut res = Vec::new();
                                    match parsed_log {
                                        MultipoolEvents::ShareTransfer(e) => {
                                            if e.to != Address::ZERO {
                                                res.push(TradingAction {
                                                    account: pg_bytes(e.to.as_slice()),
                                                    multipool: pg_bytes(
                                                        event.log.address.as_slice(),
                                                    ),
                                                    chain_id: self.chain_id as i64,
                                                    action_type: "receive".to_string(),
                                                    quanitty: e.amount.to_string(),
                                                    quote_quanitty: None,
                                                    transaction_hash: pg_bytes(txn.hash.as_slice()),
                                                    timestamp: block.timestamp as i64,
                                                });
                                            }
                                            if e.from != Address::ZERO {
                                                res.push(TradingAction {
                                                    account: pg_bytes(e.from.as_slice()),
                                                    multipool: pg_bytes(
                                                        event.log.address.as_slice(),
                                                    ),
                                                    chain_id: self.chain_id as i64,
                                                    action_type: "send".to_string(),
                                                    quanitty: e.amount.to_string(),
                                                    quote_quanitty: None,
                                                    transaction_hash: pg_bytes(txn.hash.as_slice()),
                                                    timestamp: block.timestamp as i64,
                                                });
                                            }
                                        }
                                        _ => (),
                                    }
                                    res
                                })
                            })
                            .flatten()
                            .flatten()
                            .collect::<Vec<_>>();

                        while sqlx::query("call insert_history($1);")
                            .bind::<serde_json::Value>(serde_json::to_value(&actions).unwrap())
                            .execute(&mut *pool.acquire().await?)
                            .await
                            .is_err()
                        {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
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
    }
}
