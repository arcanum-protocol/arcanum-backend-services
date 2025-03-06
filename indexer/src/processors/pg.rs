use indexer1::Processor;

use alloy::hex::ToHexExt;
use multipool_storage::storage::parse_log;
use serde_json::to_value;
use sqlx::{types::BigDecimal, Acquire, Postgres, Transaction};

pub struct PgEventProcessor;

impl Processor<Transaction<'static, Postgres>> for PgEventProcessor {
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        transaction: &mut Transaction<'static, Postgres>,
        _prev_saved_block: u64,
        _new_saved_block: u64,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        for log in logs.iter() {
            let parsed_log = parse_log(log.to_owned());
            if let Some(parsed_log) = parsed_log.map(|v| to_value(v).ok()).flatten() {
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
                    ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8);",
                )
                .bind::<BigDecimal>(chain_id.try_into()?)
                .bind(log.inner.address.to_checksum(None).to_lowercase())
                .bind::<BigDecimal>(log.block_number.unwrap().try_into()?)
                .bind::<Option<i64>>(log.block_timestamp.map(|v| v as i64))
                .bind::<String>(log.transaction_hash.unwrap().encode_hex())
                .bind::<i64>(log.log_index.unwrap().try_into()?)
                .bind(parsed_log)
                .bind(to_value(log).unwrap())
                .execute(transaction.acquire().await?)
                .await?;
            }

            //TODO: add trade insert in case
            //
            //TODO: add multipool and multipool assets management
        }

        Ok(())
    }
}
