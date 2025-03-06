use indexer1::Indexer;
use multipool::Multipool;
use multipool_indexer::processors::KafkaEventProcessor;
use sqlx::PgPool;
use std::{env, time::Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let http_url = env::var("HTTP_URL").expect("HTTP_URL must be set");
    let ws_url = env::var("WS_URL").expect("WS_URL must be set");
    let kafka_url = std::env::var("KAFKA_URL").expect("KAFKA_URL must be set");

    let pool = PgPool::connect(&database_url).await?;

    Indexer::builder()
        .pg_storage(pool)
        .http_rpc_url(http_url.parse()?)
        .ws_rpc_url(ws_url.parse()?)
        .fetch_interval(Duration::from_millis(2000))
        .filter(Multipool::filter().from_block(21974496))
        .set_processor(KafkaEventProcessor::new(
            &kafka_url,
            "chain_events".to_string(),
        ))
        .build()
        .await?
        .run()
        .await
}
