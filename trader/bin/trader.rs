use std::time::Duration;

use alloy::{
    primitives::{aliases::U96, Address},
    providers::ProviderBuilder,
    rpc::types::Filter,
    sol_types::SolEvent,
};
use indexer1::Indexer;
use multipool_indexer::EmbededProcessor;
use multipool_storage::storage::MultipoolStorage;
use multipool_trader::TraderHook;
use reqwest::Url;
use sqlx::SqlitePool;
use tokio::runtime::Handle;

const SEPOLIA_RPC_URL: &str = "";
const SEPOLIA_WS_URL: &str = "";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = SqlitePool::connect("sqlite://sqlite.db").await.unwrap();
    let th = TraderHook {
        task_timeout: Duration::from_secs(2),
        handle: Handle::current(),
        rpc: ProviderBuilder::new().on_http(Url::parse(SEPOLIA_RPC_URL).unwrap()),
    };
    let db = sled::open("test").unwrap();
    let storage = MultipoolStorage::init(db, th, Address::ZERO).await.unwrap();

    // Create a provider.
    let ws = alloy::providers::WsConnect::new(SEPOLIA_WS_URL);
    let provider = ProviderBuilder::new().on_ws(ws).await.unwrap();

    let mp = multipool_types::Multipool::deploy(provider).await.unwrap();

    mp.initialize(
        "Test".to_string(),
        "Test".to_string(),
        Address::ZERO,
        U96::from(1000),
    )
    .call()
    .await
    .unwrap();

    Indexer::builder()
        .sqlite_storage(pool)
        .http_rpc_url(Url::parse(SEPOLIA_RPC_URL).unwrap())
        .ws_rpc_url(Url::parse(SEPOLIA_WS_URL).unwrap())
        .fetch_interval(Duration::from_millis(100))
        .filter(Filter::new().events([
            multipool_types::Multipool::TargetShareChange::SIGNATURE,
            multipool_types::Multipool::AssetChange::SIGNATURE,
            multipool_types::Multipool::FeesChange::SIGNATURE,
        ]))
        .set_processor(EmbededProcessor::from_storage(storage));
    Ok(())
}
