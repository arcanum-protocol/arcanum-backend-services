use crate::processors::KafkaEventProcessor;
use backend_service::ServiceData;
use indexer1::Indexer;
use multipool::Multipool;
use multipool_types::messages::KafkaTopics;
use serde::Deserialize;
use sqlx::PgPool;
use std::time::Duration;

pub mod processors;

#[cfg(test)]
pub mod test;

#[derive(Deserialize)]
pub struct IndexerService {
    database_url: String,
    rpc_url: String,
    kafka_url: String,
    from_block: u64,
    chain_id: u64,
}

impl ServiceData for IndexerService {
    async fn run(self) -> anyhow::Result<()> {
        let pool = PgPool::connect(&self.database_url).await?;

        Indexer::builder()
            .pg_storage(pool)
            .http_rpc_url(self.rpc_url.parse()?)
            // .ws_rpc_url(ws_url.parse()?)
            .fetch_interval(Duration::from_millis(2000))
            .filter(Multipool::filter().from_block(self.from_block))
            .set_processor(KafkaEventProcessor::new(
                &self.kafka_url,
                KafkaTopics::ChainEvents(self.chain_id),
            ))
            .build()
            .await?
            .run()
            .await
    }
}
