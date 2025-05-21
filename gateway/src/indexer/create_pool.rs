use crate::service::metrics::DATABASE_REQUEST_DURATION_MS;
use alloy::primitives::Address;
use anyhow::Result;
use backend_service::KeyValue;

use sqlx::Executor;
use sqlx::Postgres;
use std::time::Instant;

pub struct MultipoolCreated {
    name: String,
    symbol: String,
    multipool: Address,
    chain_id: u64,
}

impl MultipoolCreated {
    pub fn new(multipool: Address, chain_id: u64, name: String, symbol: String) -> Self {
        Self {
            multipool,
            chain_id,
            name,
            symbol,
        }
    }

    pub async fn apply_on_storage<'a, E: Executor<'a, Database = Postgres>>(
        self,
        executor: E,
    ) -> Result<()> {
        let timer = Instant::now();
        let r = sqlx::query(
            "INSERT INTO multipools(
                chain_id,
                multipool,
                name,
                symbol,
                owner
            ) VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT (multipool) DO UPDATE
            SET
                name = $3,
                symbol = $4;
            ",
        )
        .bind::<i64>(self.chain_id.try_into()?)
        .bind::<[u8; 20]>(self.multipool.into())
        .bind::<String>(self.name)
        .bind::<String>(self.symbol)
        .bind::<[u8; 20]>(Address::ZERO.into())
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(Into::into);
        DATABASE_REQUEST_DURATION_MS.record(
            timer.elapsed().as_millis() as u64,
            &[KeyValue::new("query_name", "insert_multipool")],
        );
        r
    }
}
