use crate::{
    clickhouse::{Click, ClickhouseConfig},
    hook::TraderHook,
};
use alloy::network::EthereumWallet;
use alloy::primitives::address;
use alloy::signers::local::PrivateKeySigner;
use alloy::{primitives::Address, providers::ProviderBuilder};
use anyhow::anyhow;
use anyhow::Context;
use backend_service::ServiceData;
use cache::cache::Cache;
use multipool_storage::{pg::into_fetching_task, storage::MultipoolStorage};
use multipool_types::messages::KafkaTopics;
use reqwest::Url;
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Handle;

pub mod cache;
pub mod cashback;
pub mod clickhouse;
pub mod contracts;
pub mod execution;
pub mod hook;
pub mod strategies;
pub mod trade;
pub mod uniswap;

const FACTORY_ADDRESS: Address = address!("7eFe6656d08f2d6689Ed8ca8b5A3DEA0efaa769f");

#[derive(Deserialize)]
pub struct DbConfig {
    env_key: Option<String>,
}

#[derive(Deserialize)]
pub struct TraderService {
    rpc_url: String,
    database: Option<DbConfig>,
    pk_file: String,
    clickhouse: ClickhouseConfig,
}

impl ServiceData for TraderService {
    async fn run(self) -> anyhow::Result<()> {

        let database_env_key = self
            .database
            .map(|d| d.env_key)
            .flatten()
            .unwrap_or("DATABASE_URL".into());
        let database_url = std::env::var(&database_env_key)
            .context(format!("{} must be set", database_env_key))?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let pk =
            std::fs::read_to_string(self.pk_file).expect("Should have been able to read the file");
        let signer: PrivateKeySigner = pk.parse().expect("should parse private key");

        let wallet = EthereumWallet::from(signer);

        let rpc = ProviderBuilder::new()
            .wallet(wallet)
            .connect_http(Url::parse(&self.rpc_url).unwrap());

        let cache = Arc::new(Cache::initialize(pool, Arc::new(rpc)).await.map_err(|e| anyhow!("Failed to initialize cache {e}"))?);
        let click = Arc::new(Click::new(self.clickhouse).unwrap());
        loop {
            for mp in cache.mp_cache.iter() {
                let inn_cache = cache.clone();
                tokio::spawn(crate::hook::process_pool(inn_cache, click.clone(), mp.clone(), Duration::from_secs(10)));
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
        // let th = TraderHook {
        //     click: ,
        //     task_timeout: Duration::from_secs(2),
        //     handle: Handle::current(),
        //     rpc: ProviderBuilder::new()
        //         .wallet(wallet)
        //         .connect_http(Url::parse(&self.rpc_url).unwrap()),
        // };

        Ok(())
    }
}
