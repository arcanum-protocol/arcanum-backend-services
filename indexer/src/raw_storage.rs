use std::future::Future;

use serde::Serialize;
use sqlx::{Database, Pool, Row};

pub trait RawEventStorage {
    fn insert_event<T: Serialize + Send>(
        &self,
        contract_address: &str,
        chain_id: &str,
        block_number: u64,
        block_timestamp: u64,
        event: T,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn last_observed_block_number(
        &self,
        chain_id: &str,
    ) -> impl Future<Output = anyhow::Result<i64>> + Send;

    fn update_last_observed_block_number(
        &self,
        chain_id: &str,
        block_number: u64,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

#[derive(Clone)]
pub struct RawEventStorageImpl<DB: Database> {
    pool: Pool<DB>,
}

impl<DB: Database> RawEventStorageImpl<DB> {
    pub fn new(pool: Pool<DB>) -> Self {
        Self { pool }
    }
}

impl RawEventStorage for RawEventStorageImpl<sqlx::Postgres> {
    // TODO insert events in bulk
    async fn insert_event<T: Serialize + Send>(
        &self,
        contract_address: &str,
        chain_id: &str,
        block_number: u64,
        block_timestamp: u64,
        event: T,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "
            INSERT INTO raw_events (contract_address, chain_id, block_number, block_timestamp, event)
            VALUES ($1, $2, $3, $4, $5)
            ",
        )
        .bind(contract_address)
        .bind(chain_id)
        .bind::<i64>(block_number.try_into()?)
        .bind::<i64>(block_timestamp.try_into()?)
        .bind(serde_json::to_value(&event)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn last_observed_block_number(&self, chain_id: &str) -> anyhow::Result<i64> {
        let row = sqlx::query("SELECT last_observed_block FROM chains WHERE chain_id = $1")
            .bind(chain_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get(0))
    }

    async fn update_last_observed_block_number(
        &self,
        chain_id: &str,
        block_number: u64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "
                UPDATE chains
                SET last_observed_block = $1
                WHERE chain_id = $2
            ",
        )
        .bind::<i64>(block_number.try_into()?)
        .bind(chain_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
