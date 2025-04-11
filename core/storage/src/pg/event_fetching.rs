use std::time::Duration;

use serde_json::{from_value, Value};
use sqlx::prelude::FromRow;
use sqlx::{postgres::PgRow, PgPool, Row};

use crate::hook::HookInitializer;
use crate::storage::MultipoolStorage;
use multipool_types::messages::Block;

#[derive(FromRow)]
pub struct BlocksData {
    block: Value,
}

pub async fn into_fetching_task<HI: HookInitializer>(
    storage: &mut MultipoolStorage<HI>,
    pool: PgPool,
    interval: Duration,
    chain_ids: Vec<u64>,
) -> anyhow::Result<()> {
    loop {
        for chain_id in chain_ids.iter() {
            let last_seen_block = storage.get_last_seen_block(chain_id)?.unwrap_or(0);
            let blocks: Vec<Block> = sqlx::query_as(
                "
            SELECT 
                block 
            FROM 
                blocks 
            WHERE 
                block_number > $1 
                and chain_id = $2 
            ORDER BY block_number ASC;",
            )
            .bind::<i64>(last_seen_block.try_into()?)
            .bind::<i64>((*chain_id).try_into()?)
            .fetch_all(&mut *pool.acquire().await?)
            .await?
            .into_iter()
            .map(|v: BlocksData| from_value(v.block).unwrap())
            .collect();
            storage
                .create_multipools(chain_id, blocks.as_slice().try_into()?)
                .await?;
            storage
                .apply_events(chain_id, blocks.as_slice().try_into()?)
                .await?;
        }

        tokio::time::sleep(interval).await;
    }
}
