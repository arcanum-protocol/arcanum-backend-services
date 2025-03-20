use std::ops::Div;
use std::str::FromStr;
use std::time::Duration;

use crate::hook::HookInitializer;
use crate::price_fetch::get_asset_prices;
use alloy::providers::Provider;
use anyhow::anyhow;
use multipool::expiry::{MayBeExpired, StdTimeExtractor};
use sqlx::types::BigDecimal;
use sqlx::PgPool;

#[derive(Clone)]
pub struct PricePush<P: Provider + Clone + 'static> {
    pool: PgPool,
    delay: Duration,
    multicall_chunk_size: usize,
    rpc: P,
}

impl<P: Provider + Clone + 'static> HookInitializer for PricePush<P> {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let instance = self.clone();
        vec![tokio::spawn(async move {
            loop {
                let mut mp = multipool();
                let asset_prices = get_asset_prices(
                    mp.contract_address(),
                    mp.asset_list(),
                    instance.multicall_chunk_size,
                    &instance.rpc,
                )
                .await?;
                mp.update_prices(
                    &asset_prices
                        .into_iter()
                        .map(|(address, price)| {
                            (address, MayBeExpired::build::<StdTimeExtractor>(price))
                        })
                        .collect(),
                );
                let price = mp
                    .get_price(&mp.contract_address())
                    .map_err(|_| anyhow!("failed to get price"))?
                    .not_older_than::<StdTimeExtractor>(60)
                    .ok_or(anyhow!("price expired"))
                    .map(|p| {
                        BigDecimal::from_str(&p.to_string()).map(|p| {
                            p.div(BigDecimal::from_str("79228162514264337593543950336").unwrap())
                        })
                    })??;

                sqlx::query("call assemble_stats($1,$2)")
                    .bind(mp.contract_address().to_checksum(None).to_lowercase())
                    .bind(price)
                    .execute(&mut *instance.pool.acquire().await?)
                    .await?;
                tokio::time::sleep(instance.delay).await;
            }
        })]
    }
}
