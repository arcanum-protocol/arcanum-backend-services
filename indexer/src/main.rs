use std::{future::ready, time::Duration};

use alloy::{rpc::types::Filter, sol_types::SolEvent};
use anyhow::Result;
use indexer1::{Indexer, Processor};
use multipool::Multipool;
use multipool_storage::{hook::HookInitializer, storage::MultipoolStorage};

#[cfg(test)]
pub mod test;

pub struct EmptyHookInitialiser;

impl HookInitializer for EmptyHookInitialiser {
    async fn initialize_hook<F: Fn() -> Multipool>(
        &mut self,
        _getter: F,
    ) -> tokio::task::JoinHandle<Result<()>> {
        tokio::spawn(ready(Ok(())))
    }
}

pub struct EmbededProcessor {
    storage: MultipoolStorage<EmptyHookInitialiser>,
}

impl Processor for EmbededProcessor {
    async fn process<DB: indexer1::sqlx::Database>(
        &mut self,
        logs: &[alloy::rpc::types::Log],
        transaction: &mut indexer1::sqlx::Transaction<'static, DB>,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        self.storage
            .apply_events(
                logs.into_iter().cloned().map(|l| alloy::primitives::Log {
                    address: l.inner.address,
                    data: l.inner.data,
                }),
                0,
                1,
            )
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    //let sqlite = sqlx::SqlitePool::;
    Indexer::builder()
        .sqlite_storage(pool)
        .http_rpc_url("https://")
        .ws_rpc_url("https://")
        .fetch_interval(Duration::from_millis(100))
        .filter(Filter::new().events([
            multipool_types::Multipool::TargetShareChange::SIGNATURE,
            multipool_types::Multipool::AssetChange::SIGNATURE,
            multipool_types::Multipool::FeesChange::SIGNATURE,
        ]))
        .set_processor(EmbededProcessor)
}
