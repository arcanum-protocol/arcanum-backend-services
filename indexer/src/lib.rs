use std::{future::ready, time::Duration};

use anyhow::Result;
use indexer1::Processor;
use multipool::Multipool;
use multipool_storage::{hook::HookInitializer, storage::MultipoolStorage};

#[cfg(test)]
pub mod test;

pub struct EmptyHookInitialiser;

impl HookInitializer for EmptyHookInitialiser {
    async fn initialize_hook<F: Fn() -> Multipool + Send + 'static>(
        &mut self,
        multipool: F,
    ) -> tokio::task::JoinHandle<Result<()>> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    }
}

pub struct EmbededProcessor<T: HookInitializer> {
    storage: MultipoolStorage<T>,
}

impl<T: HookInitializer> EmbededProcessor<T> {
    pub fn from_storage(storage: MultipoolStorage<T>) -> Self {
        Self { storage }
    }
}

impl<T, R: HookInitializer> Processor<T> for EmbededProcessor<R> {
    async fn process(
        &mut self,
        logs: &[alloy::rpc::types::Log],
        _transaction: &mut T,
        prev_saved_block: u64,
        new_saved_block: u64,
        _chain_id: u64,
    ) -> anyhow::Result<()> {
        self.storage
            .apply_events(logs.into_iter().cloned(), prev_saved_block, new_saved_block)
            .await?;
        Ok(())
    }
}
