use std::time::Duration;

use alloy::providers::Provider;
use anyhow::anyhow;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};

use indexer1::Processor;
use multipool_types::messages::{Blocks, KafkaTopics, MsgPack};
use sqlx::{Postgres, Transaction};

pub struct KafkaEventProcessor<P: Provider + Clone + 'static> {
    rpc: P,
    topic: KafkaTopics,
    producer: FutureProducer,
}

impl<P: Provider + Clone + 'static> KafkaEventProcessor<P> {
    pub fn new(kafka_url: &str, topic: KafkaTopics, rpc: P) -> Self {
        Self {
            topic,
            rpc,
            producer: ClientConfig::new()
                .set("bootstrap.servers", kafka_url)
                .create()
                .expect("Cannot create kafka producer"),
        }
    }
}

impl<P: Provider + Clone + 'static> Processor<Transaction<'static, Postgres>>
    for KafkaEventProcessor<P>
{
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        _transaction: &mut Transaction<'static, Postgres>,
        _prev_saved_block: u64,
        _new_saved_block: u64,
        _chain_id: u64,
    ) -> anyhow::Result<()> {
        let blocks = Blocks::parse_logs(logs, self.rpc.clone())
            .await
            .map_err(|_e| anyhow!("ParseLogsErrror"))?;
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
