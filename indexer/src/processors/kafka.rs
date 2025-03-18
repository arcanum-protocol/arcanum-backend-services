use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use multipool_types::kafka::{ChainBlock, ChainEvent};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};

use alloy::hex::ToHexExt;
use indexer1::Processor;
use multipool_storage::storage::parse_log;
use multipool_types::kafka::KafkaTopics;
use serde_json::to_value;
use sqlx::{Postgres, Transaction};

pub struct KafkaEventProcessor {
    topic: KafkaTopics,
    producer: FutureProducer,
}

impl KafkaEventProcessor {
    pub fn new(kafka_url: &str, topic: KafkaTopics) -> Self {
        Self {
            topic,
            producer: ClientConfig::new()
                .set("bootstrap.servers", kafka_url)
                .create()
                .expect("Cannot create kafka producer"),
        }
    }
}

impl Processor<Transaction<'static, Postgres>> for KafkaEventProcessor {
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        _transaction: &mut Transaction<'static, Postgres>,
        _prev_saved_block: u64,
        _new_saved_block: u64,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        let mut map = HashMap::new();
        for l in logs.iter() {
            map.entry(l.block_number.unwrap()).or_insert(vec![]).push(l);
        }
        for (block_number, logs) in map.into_iter() {
            let block_timestamp = logs[0].block_timestamp;
            let events: Vec<ChainEvent> = logs
                .into_iter()
                .map(|log| ChainEvent {
                    emitter_address: log.inner.address.to_checksum(None).to_lowercase(),
                    transaction_hash: log.transaction_hash.unwrap().encode_hex(),
                    event_index: log.log_index,
                    parsed_event: parse_log(log.to_owned())
                        .and_then(|v| to_value(v).ok()),
                    row_event: log.clone(),
                })
                .collect();
            let block = ChainBlock {
                chain_id,
                block_number,
                block_timestamp,
                events,
            };
            self.producer
                .send(
                    // Somehow fix all transitions
                    FutureRecord::to(self.topic.as_ref())
                        .key(&format!("{}|{}", chain_id, block_number,))
                        .payload(&to_value(&block).unwrap().to_string()),
                    Duration::from_secs(1),
                )
                .await
                .map_err(|(e, msg)| {
                    anyhow!("Failed to send log to kafka: {e} ; \n Message: {:?}", msg)
                })?;
            println!("APPLIED BLOCK {:?}", block);
        }
        Ok(())
    }
}
