use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};

use alloy::hex::ToHexExt;
use indexer1::Processor;
use multipool_storage::storage::parse_log;
use serde_json::{json, to_value, Value};
use sqlx::{Postgres, Transaction};

pub struct KafkaEventProcessor {
    topic: String,
    producer: FutureProducer,
}

impl KafkaEventProcessor {
    pub fn new(kafka_url: &str, topic: String) -> Self {
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
        for l in logs.into_iter() {
            map.entry(l.block_number.unwrap()).or_insert(vec![]).push(l);
        }
        for (block_number, logs) in map.into_iter() {
            let logs: Value = logs
                .into_iter()
                .map(|log| {
                    json!({
                        "chain_id": chain_id,
                        "emitter_address": log.inner.address.to_checksum(None).to_lowercase(),
                        "block_number": log.block_number,
                        "block_timestamp": log.block_timestamp,
                        "transaction_hash": log.transaction_hash.unwrap().encode_hex(),
                        "event_index": log.log_index,
                        "event": parse_log(log.to_owned()),
                        "row_event": to_value(log).unwrap()
                    })
                })
                .collect();
            self.producer
                .send(
                    // Somehow fix all transitions
                    FutureRecord::to(&self.topic)
                        .key(&format!("{}{}", chain_id, block_number,))
                        .payload(&logs.to_string()),
                    Duration::from_secs(1),
                )
                .await
                .map_err(|(e, msg)| {
                    anyhow!("Failed to send log to kafka: {e} ; \n Message: {:?}", msg)
                })?;
        }
        Ok(())
    }
}
