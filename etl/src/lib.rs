use std::sync::Arc;
use std::time::Duration;

use alloy::hex::ToHexExt;
use alloy::primitives::address;
use alloy::sol_types::SolEventInterface;
use alloy::{primitives::Address, providers::ProviderBuilder};
use backend_service::ServiceData;
use multipool_storage::storage::{
    parse_log, MultipoolStorage, MultipoolUpdates, MultipoolsUpdates,
};
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
use tokio::sync::RwLock;

use crate::processors::kafka::Etl;

mod processors;

const FACTORY_ADDRESS: Address = address!("7eFe6656d08f2d6689Ed8ca8b5A3DEA0efaa769f");

#[derive(Deserialize)]
pub struct EtlService {
    kafka_url: String,
    kafka_group: String,
    chain_id: u64,
    database_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TradingAction {
    account: String,
    multipool: String,
    chain_id: i64,
    action_type: String,
    quantity: String,
    quote_quantity: Option<String>,
    transaction_hash: String,
    timestamp: i64,
}

fn pg_bytes(bytes: &[u8]) -> String {
    format!("\\x{}", bytes.encode_hex())
}

impl ServiceData for EtlService {
    async fn run(self) -> anyhow::Result<()> {
        // let producer: FutureProducer = ClientConfig::new()
        //     .set("bootstrap.servers", &self.kafka_url)
        //     .create()
        //     .expect("Cannot create kafka producer");

        let pool = PgPool::connect(&self.database_url).await?;

        let th = Etl {
            // producer,
            delay: Duration::from_secs(2),
            chain_id: self.chain_id,
            pool: pool.clone(),
        };
        let db = sled::open("etl_sled_db").unwrap();
        let storage = Arc::new(RwLock::new(
            MultipoolStorage::init(db, th, FACTORY_ADDRESS)
                .await
                .unwrap(),
        ));

        let events_consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &self.kafka_group)
            .set("bootstrap.servers", &self.kafka_url)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Creation failed");

        let prices_consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &self.kafka_group)
            .set("bootstrap.servers", &self.kafka_url)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Creation failed");

        events_consumer
            .subscribe(&[KafkaTopics::ChainEvents(self.chain_id).to_string().as_str()])
            .expect("Failed to subscribe to topic");

        prices_consumer
            .subscribe(&[KafkaTopics::MpPrices(self.chain_id).to_string().as_str()])
            .expect("Failed to subscribe to topic");
        let inn_storage = storage.clone();

        tokio::spawn(async move {
            let mut stream = prices_consumer.stream();
            while let Some(Ok(message)) = stream.next().await {
                match message.topic().try_into().unwrap() {
                    KafkaTopics::MpPrices(_chain_id) => {
                        let bytes = message
                            .payload()
                            .context(anyhow!("Received message with no payload"))
                            .unwrap();
                        let data = PriceData::unpack(bytes);
                        inn_storage
                            .read()
                            .await
                            .apply_prices(data.address, data.prices)
                            .await
                            .unwrap();
                    }
                    _ => unreachable!(),
                }
                prices_consumer
                    .commit_message(&message, CommitMode::Sync)
                    .unwrap();
            }
        });

        loop {
            let mut stream = events_consumer.stream();
            // add better error handling
            while let Some(Ok(message)) = stream.next().await {
                // println!("TOPIC!!!!!!!       _________________ {:?}", message.topic());
                match message.topic().try_into()? {
                    KafkaTopics::ChainEvents(chain_id) => {
                        let bytes = message
                            .payload()
                            .context(anyhow!("Received message with no payload"))?;
                        let block = messages::Block::unpack(bytes);
                        let blocks: [messages::Block; 1] = [block.clone()];

                        storage
                            .write()
                            .await
                            .create_multipools(blocks.as_slice().try_into()?, chain_id)
                            .await?;

                        storage
                            .read()
                            .await
                            .apply_events(blocks.as_slice().try_into()?)
                            .await?;

                        let actions: Vec<TradingAction> = block
                            .transactions
                            .iter()
                            .map(|txn| {
                                txn.events.iter().map(|event| {
                                    let mut res = Vec::new();

                                    if let Ok(parsed_log) =
                                        MultipoolEvents::decode_log(&event.log, false)
                                    {
                                        match parsed_log.data {
                                            MultipoolEvents::ShareTransfer(e) => {
                                                if e.to != Address::ZERO {
                                                    res.push(TradingAction {
                                                        account: pg_bytes(e.to.as_slice()),
                                                        multipool: pg_bytes(
                                                            event.log.address.as_slice(),
                                                        ),
                                                        chain_id: self.chain_id as i64,
                                                        action_type: "receive".to_string(),
                                                        quantity: e.amount.to_string(),
                                                        quote_quantity: None,
                                                        transaction_hash: pg_bytes(
                                                            txn.hash.as_slice(),
                                                        ),
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
                                                        quantity: e.amount.to_string(),
                                                        quote_quantity: None,
                                                        transaction_hash: pg_bytes(
                                                            txn.hash.as_slice(),
                                                        ),
                                                        timestamp: block.timestamp as i64,
                                                    });
                                                }
                                            }
                                            _ => (),
                                        }
                                    }
                                    res
                                })
                            })
                            .flatten()
                            .flatten()
                            .collect::<Vec<_>>();
                        while let Err(e) = sqlx::query("call insert_history($1::JSON);")
                            .bind::<serde_json::Value>(serde_json::to_value(&actions).unwrap())
                            .execute(&mut *pool.acquire().await?)
                            .await
                        {
                            println!("{actions:?}");
                            println!("{e}");
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
                    }
                    _ => unreachable!(),
                }
                events_consumer.commit_message(&message, CommitMode::Sync)?;
            }
        }
    }
}
