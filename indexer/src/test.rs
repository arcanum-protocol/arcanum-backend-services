use std::time::Duration;

use alloy::{
    node_bindings::Anvil,
    primitives::{aliases::U96, Address},
    providers::ProviderBuilder,
    rpc::types::Filter,
    sol_types::SolEvent,
};
use anyhow::Result;
use indexer1::Indexer;
use multipool_storage::storage::MultipoolStorage;

use crate::{EmbededProcessor, EmptyHookInitialiser};

#[sqlx::test]
async fn happy_path(pool: sqlx::SqlitePool) -> Result<()> {
    let db = sled::open("test")?;
    let storage = MultipoolStorage::init(db, EmptyHookInitialiser, Address::ZERO).await?;

    let anvil = Anvil::new().block_time(1).try_spawn()?;

    // Create a provider.
    let ws = alloy::providers::WsConnect::new(anvil.ws_endpoint());
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let mp = multipool_types::Multipool::deploy(provider).await?;

    mp.initialize(
        "Test".to_string(),
        "Test".to_string(),
        Address::ZERO,
        U96::from(1000),
    )
    .call()
    .await?;

    Indexer::builder()
        .sqlite_storage(pool)
        .http_rpc_url(anvil.endpoint_url())
        .ws_rpc_url(anvil.ws_endpoint_url())
        .fetch_interval(Duration::from_millis(100))
        .filter(Filter::new().events([
            multipool_types::Multipool::TargetShareChange::SIGNATURE,
            multipool_types::Multipool::AssetChange::SIGNATURE,
            multipool_types::Multipool::FeesChange::SIGNATURE,
        ]))
        .set_processor(EmbededProcessor::from_storage(storage));
    Ok(())
}
