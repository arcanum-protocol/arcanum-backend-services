use alloy::providers::Provider;
use alloy::{primitives::Address, providers::ProviderBuilder};
use backend_service::ServiceData;
use processor::get_mps_prices;
use reqwest::Url;
use serde::Deserialize;
use sqlx::types::BigDecimal;
use sqlx::PgPool;
use sqlx::Row;
use std::str::FromStr;
use std::time::Duration;

mod processor;

#[derive(Deserialize)]
pub struct PriceFetcherService {
    rpc_url: String,
    db_url: String,
    block_delay: u64,
    chain_id: i64,
}

impl ServiceData for PriceFetcherService {
    async fn run(self) -> anyhow::Result<()> {
        let pool = PgPool::connect(&self.db_url).await?;
        let provider = ProviderBuilder::new().on_http(Url::parse(&self.rpc_url).unwrap());
        let mut last_checked_block = provider.get_block_number().await?;
        loop {
            let new_block = provider.get_block_number().await?;
            if last_checked_block + self.block_delay >= new_block {
                let mps: Vec<Address> =
                    sqlx::query("SELECT multipool FROM multipools WHERE chain_id = $1")
                        .bind::<i64>(self.chain_id.try_into().unwrap())
                        .fetch_all(&mut *pool.acquire().await?)
                        .await?
                        .into_iter()
                        .map(|a| {
                            let bytes: Vec<u8> = a.try_get("multipool").unwrap();
                            Address::from_slice(bytes.as_slice())
                        })
                        .collect();
                let (prices, ts) = get_mps_prices(mps, &provider).await?;
                for (mp, price) in prices.into_iter() {
                    sqlx::query("call insert_price($1, $2, $3, $4)")
                        .bind(self.chain_id)
                        .bind::<&[u8]>(mp.as_slice())
                        .bind::<i64>(ts.to())
                        .bind::<BigDecimal>(BigDecimal::from_str(price.to_string().as_str())?)
                        .execute(&mut *pool.acquire().await?)
                        .await?;
                }
                last_checked_block = new_block;
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
}
