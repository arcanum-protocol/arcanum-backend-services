use alloy::primitives::{address, Address};
use anyhow::Result;
use futures::future::ready;
use multipool_storage::{hook::HookInitializer, storage::MultipoolStorage};

pub struct TestHookInitializer;

impl HookInitializer for TestHookInitializer {
    async fn initialize_hook<F: Fn() -> multipool::Multipool>(
        &mut self,
        _getter: F,
    ) -> Vec<tokio::task::JoinHandle<Result<()>> {
        vec![tokio::spawn(ready(Ok(())))]
    }
}

#[tokio::test]
async fn happy_path() -> Result<()> {
    let db = sled::open("test")?;
    let mut storage = MultipoolStorage::init(db, TestHookInitializer, Address::ZERO).await?;
    storage.apply_events(vec![], 1, 3).await?;
    Ok(())
}
