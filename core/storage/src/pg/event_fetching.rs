use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};

use crate::hook::HookInitializer;
use crate::storage::MultipoolStorage;

pub async fn into_fetching_task<HI: HookInitializer>(
    storage: &mut MultipoolStorage<HI>,
    pool: PgPool,
) -> anyhow::Result<()> {
    loop {
        let last_seen_block = storage.get_last_seen_block()?.unwrap_or(0);
        let logs = sqlx::query(
            "SELECT row_event FROM events WHERE block_number > $1 ORDER BY block_number ASC;",
        )
        .bind::<i64>(last_seen_block.try_into()?)
        .fetch_all(&mut *pool.acquire().await?)
        .await?
        .into_iter()
        .map(|v: PgRow| serde_json::from_value(v.get::<Value, _>("row_event")).unwrap());

        storage
            .apply_events(logs, last_seen_block + 1, None)
            .await?;
    }
}
