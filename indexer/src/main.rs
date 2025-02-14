use std::{env, time::Duration};

use alloy::hex::ToHex;
use indexer1::{Indexer, Processor};
use multipool::Multipool;
use multipool_storage::storage::parse_log;
use serde_json::to_value;
use sqlx::{types::BigDecimal, Acquire, PgPool, Postgres, Transaction};

pub struct PgEventProcessor;

impl Processor<Transaction<'static, Postgres>> for PgEventProcessor {
    async fn process(
        &mut self,
        logs: &[alloy::rpc::types::Log],
        transaction: &mut Transaction<'static, Postgres>,
        _prev_saved_block: u64,
        _new_saved_block: u64,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        for (log, parsed_log) in logs
            .iter()
            .map(|log| (log, parse_log(log.to_owned()).unwrap()))
        {
            sqlx::query(
                "INSERT INTO events(
                chain_id, 
                emitter_address, 
                block_number, 
                block_timestamp, 
                transaction_hash, 
                event_index, 
                event,
                row_event
                ) VALUES ($1,$2,$3,$4,$5,$6,$7);",
            )
            .bind::<BigDecimal>(chain_id.try_into()?)
            .bind(log.inner.address.to_checksum(None).to_lowercase())
            .bind::<BigDecimal>(log.block_number.unwrap().try_into()?)
            .bind::<i64>(log.block_timestamp.unwrap().try_into()?)
            .bind::<String>(log.transaction_hash.unwrap().encode_hex())
            .bind::<i64>(log.log_index.unwrap().try_into()?)
            .bind(to_value(parsed_log).unwrap())
            .bind(to_value(log).unwrap())
            .execute(transaction.acquire().await?)
            .await?;

            //TODO: add trade insert in case
            //
            //TODO: add multipool and multipool assets management
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    let http_url = env::var("HTTP_URL").expect("HTTP_URL must be set");
    let ws_url = env::var("WS_URL").expect("WS_URL must be set");

    Indexer::builder()
        .pg_storage(pool)
        .http_rpc_url(http_url.parse()?)
        .ws_rpc_url(ws_url.parse()?)
        .fetch_interval(Duration::from_millis(100))
        .filter(Multipool::filter())
        .set_processor(PgEventProcessor)
        .build()
        .await?
        .run()
        .await
}
