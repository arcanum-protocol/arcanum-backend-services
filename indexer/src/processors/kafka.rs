use std::time::Duration;

use anyhow::anyhow;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};

use indexer1::Processor;
use multipool_types::messages::{Blocks, KafkaTopics, MsgPack};
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
        _chain_id: u64,
    ) -> anyhow::Result<()> {
        let blocks = Blocks::try_from(logs).map_err(|_e| anyhow!("ParseErrror"))?;
        for block in blocks.0 {
            self.producer
                .send(
                    // Somehow fix all transitions
                    FutureRecord::to(self.topic.to_string().as_str())
                        .key(&block.number.to_string())
                        .payload(&block.pack()),
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
