use std::time::Duration;

use anyhow::Result;
use indexer1::Processor;
use multipool::Multipool;
use multipool_storage::{hook::HookInitializer, storage::MultipoolStorage};

pub struct EmbededProcessor<T: HookInitializer> {
    pub storage: MultipoolStorage<T>,
}

impl<T: HookInitializer> EmbededProcessor<T> {
    pub fn from_storage(storage: MultipoolStorage<T>) -> Self {
        println!("processor initialized");
        Self { storage }
    }
}

impl<T, R: HookInitializer> Processor<T> for EmbededProcessor<R> {
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        _transaction: &mut T,
        _prev_saved_block: u64,
        _new_saved_block: u64,
        _chain_id: u64,
    ) -> anyhow::Result<()> {
        println!("logs {:?}", logs);
        // self.storage
        //     .apply_events(
        //         logs.into_iter().cloned(),
        //         prev_saved_block,
        //         Some(new_saved_block),
        //     )
        //     .await?;
        Ok(())
    }
}
