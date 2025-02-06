use std::future::ready;

use anyhow::Result;
use indexer1::Processor;
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

impl EmbededProcessor {
    pub fn from_storage(storage: MultipoolStorage<EmptyHookInitialiser>) -> Self {
        Self { storage }
    }
}

impl<T> Processor<T> for EmbededProcessor {
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

#[tokio::main]
async fn main() -> Result<()> {
    Ok(())
}
