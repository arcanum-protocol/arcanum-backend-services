use std::{sync::Arc, time::Duration};

use alloy::node_bindings::Anvil;
use alloy::primitives::address;
use alloy::{
    primitives::Address, providers::ProviderBuilder, rpc::types::Filter, sol_types::SolEvent,
};
use indexer1::Indexer;
use multipool::Multipool;
use multipool_indexer::EmbededProcessor;
use multipool_storage::storage::MultipoolStorage;
use multipool_trader::{clickhouse::Click, TraderHook};
use reqwest::Url;
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};
use tokio::runtime::Handle;

// const SEPOLIA_RPC_URL: &str = "https://eth-sepolia.g.alchemy.com/v2/c_34X8mrHf2CeUbKJyRn9El7loLauTbU";
const SEPOLIA_RPC_URL: &str = "http://127.0.0.1:8545";
const DB_URL: &str = "sqlite://sqlite.db";
const FACTORY_ADDRESS: Address = address!("e9db1A8baD7089a64a73507FB084Bc902CC41dE7");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }
    let pool = SqlitePool::connect(DB_URL).await.unwrap();
    let th = TraderHook {
        click: Arc::new(Click::new().unwrap()),
        task_timeout: Duration::from_secs(2),
        handle: Handle::current(),
        rpc: ProviderBuilder::new().on_http(Url::parse(SEPOLIA_RPC_URL).unwrap()),
    };
    let db = sled::open("sled_db").unwrap();
    let storage = MultipoolStorage::init(db, th, FACTORY_ADDRESS)
        .await
        .unwrap();

    // Create a provider.

    Indexer::builder()
        .sqlite_storage(pool)
        .http_rpc_url(Url::parse(SEPOLIA_RPC_URL).unwrap())
        // .ws_rpc_url(Url::parse(SEPOLIA_WS_URL).unwrap())
        .fetch_interval(Duration::from_millis(100))
        .filter(Multipool::filter().from_block(21815212))
        .set_processor(EmbededProcessor::from_storage(storage))
        .build()
        .await
        .unwrap()
        .run()
        .await
        .unwrap();
    Ok(())
}
