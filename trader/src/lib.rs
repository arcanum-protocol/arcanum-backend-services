use backend_service::ServiceData;
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use anyhow::Context;

use crate::{
    clickhouse::{Click, ClickhouseConfig},
    hook::TraderHook,
};
use alloy::network::EthereumWallet;
use alloy::primitives::address;
use alloy::signers::local::PrivateKeySigner;
use alloy::{primitives::Address, providers::ProviderBuilder};
use multipool_storage::{pg::into_fetching_task, storage::MultipoolStorage};
use multipool_types::messages::KafkaTopics;
use reqwest::Url;
use tokio::runtime::Handle;

pub mod cashback;
pub mod cache;
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
    chain_id: u64,
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
        let database_url =
            std::env::var(&database_env_key).context(format!("{} must be set", database_env_key))?;
        let pool = sqlx::PgPool::connect(&database_url).await?;
        let pk =
            std::fs::read_to_string(self.pk_file).expect("Should have been able to read the file");
        let signer: PrivateKeySigner = pk.parse().expect("should parse private key");

        let wallet = EthereumWallet::from(signer);

        let th = TraderHook {
            click: Arc::new(Click::new(self.clickhouse).unwrap()),
            task_timeout: Duration::from_secs(2),
            handle: Handle::current(),
            rpc: ProviderBuilder::new()
                .wallet(wallet)
                .on_http(Url::parse(&self.rpc_url).unwrap()),
        };
        let db = sled::open("sled_db").unwrap();
        let mut storage = MultipoolStorage::init(db, th, FACTORY_ADDRESS)
            .await
            .unwrap();

        into_fetching_task(&mut storage, pool, Duration::from_secs(1), vec![self.chain_id]).await?;

        Ok(())
    }
}
