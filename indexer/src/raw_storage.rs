use std::future::Future;

use serde::Serialize;
use sqlx::{Database, Pool};

pub trait RawEventStorage {
    fn insert_event<T: Serialize + Send>(
        &self,
        contract_address: &str,
        chain_id: &str,
        block_number: i64,
        event: T,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub struct RawEventStorageImpl<DB: Database> {
    pool: Pool<DB>,
}

impl<DB: Database> RawEventStorageImpl<DB> {
    pub fn new(pool: Pool<DB>) -> Self {
        Self { pool }
    }
}

impl RawEventStorage for RawEventStorageImpl<sqlx::Postgres> {
    async fn insert_event<T: Serialize + Send>(
        &self,
        contract_address: &str,
        chain_id: &str,
        block_number: i64,
        event: T,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "
            INSERT INTO raw_events (contract_address, chain_id, block_number, event)
            VALUES ($1, $2, $3, $4)
            ",
        )
        .bind(contract_address)
        .bind(chain_id)
        .bind(block_number)
        .bind(serde_json::to_value(&event)?)
        .fetch_all(&self.pool)
        .await?;

        Ok(())
    }
}
