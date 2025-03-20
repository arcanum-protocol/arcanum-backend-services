use indexer1::Indexer;
use multipool::Multipool;
use multipool_indexer::processors::KafkaEventProcessor;
use multipool_types::messages::KafkaTopics;
use sqlx::PgPool;
use std::{env, time::Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let http_url = env::var("HTTP_URL").expect("HTTP_URL must be set");
    // let ws_url = env::var("WS_URL").expect("WS_URL must be set");
    let kafka_url = std::env::var("KAFKA_URL").expect("KAFKA_URL must be set");
    let from_block = std::env::var("FROM_BLOCK").expect("FROM_BLOCK must be set");
    let chain_id = std::env::var("CHAIN_ID").expect("CHAIN_ID must be set");

    let pool = PgPool::connect(&database_url).await?;

    Indexer::builder()
        .pg_storage(pool)
        .http_rpc_url(http_url.parse()?)
        // .ws_rpc_url(ws_url.parse()?)
        .fetch_interval(Duration::from_millis(2000))
        .filter(
            Multipool::filter()
                .from_block(from_block.parse::<u64>().expect("Invalid block format")),
        )
        .set_processor(KafkaEventProcessor::new(
            &kafka_url,
            KafkaTopics::ChainEvents(chain_id.parse().unwrap()),
        ))
        .build()
        .await?
        .run()
        .await
}
