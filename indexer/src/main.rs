use std::path::PathBuf;

use alloy::providers::{ProviderBuilder, WsConnect};
use clap::Parser;
use multipool_storage::MultipoolStorage;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::fs;

mod contracts;
mod indexer;
mod multipool_storage;
mod raw_storage;

#[derive(Parser, Debug)]
struct Args {
    config_path: PathBuf,
    from_block: Option<u64>,
}

#[derive(Deserialize)]
struct IndexerConfig {
    provider_url: String,
    ws_provider_url: Option<String>,
    factory_contract_address: String,
    poll_interval_millis: u64,
    raw_storage_uri: String,
    multipool_storage_path: String,
    multipool_contract_bytecode: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let config_content = fs::read_to_string(args.config_path).await?;
    let deserialized_config: IndexerConfig = serde_yml::from_str(&config_content)?;

    let provider = ProviderBuilder::new().on_http(deserialized_config.provider_url.parse()?);
    let ws_provider = match deserialized_config.ws_provider_url {
        Some(ws_provider_url) => Some(
            ProviderBuilder::new()
                .on_ws(WsConnect::new(ws_provider_url))
                .await?,
        ),
        None => None,
    };

    let mp_storage_db =
        MultipoolStorage::new(sled::open(deserialized_config.multipool_storage_path)?);
    let pg_pool = PgPool::connect(&deserialized_config.raw_storage_uri).await?;
    let raw_storage = raw_storage::RawEventStorageImpl::new(pg_pool);

    let indexer = indexer::MultipoolIndexer::new(
        deserialized_config.factory_contract_address.parse()?,
        deserialized_config.multipool_contract_bytecode,
        provider,
        ws_provider,
        args.from_block,
        raw_storage,
        mp_storage_db,
        deserialized_config.poll_interval_millis,
    )
    .await?;
    indexer.run().await?;

    Ok(())
}
