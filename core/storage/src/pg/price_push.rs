use std::collections::HashMap;
use std::ops::Div;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};

use crate::hook::HookInitializer;
use alloy::providers::Provider;
use anyhow::anyhow;
use itertools::Itertools;
use multipool::expiry::StdTimeExtractor;
use sqlx::types::BigDecimal;
use sqlx::PgPool;

#[derive(Clone)]
pub struct PricePush<P: Provider + Clone + 'static> {
    pool: PgPool,
    delay: Duration,
    multicall_chunk_size: usize,
    multicall_address: Address,
    // rpc: RootProvider<Http<Client>>,
    rpc: P,
}

impl<P: Provider + Clone + 'static> PricePush<P> {
    pub async fn get_asset_prices(
        &self,
        mp: Address,
        assets: Vec<Address>,
    ) -> anyhow::Result<HashMap<Address, U256>> {
        let multipool_functions = multipool_types::Multipool::abi::functions();
        let get_price_func = &multipool_functions.get("getPrice").unwrap()[0];

        let mut prices = Vec::new();
        let chunked_assets = assets
            .iter()
            .chunks(self.multicall_chunk_size)
            .into_iter()
            .map(|chunk| chunk.into_iter().collect_vec())
            .collect_vec();
        for chunk in chunked_assets {
            let mut mc = alloy_multicall::Multicall::new(&self.rpc, self.multicall_address);
            for asset in chunk {
                mc.add_call(mp, get_price_func, &[DynSolValue::Address(*asset)], true);
            }
            let result = mc
                .call()
                .await?
                .into_iter()
                .map(|p| p.unwrap().as_uint().unwrap().0);
            prices.extend(result);
        }
        Ok(assets.into_iter().zip(prices.into_iter()).collect())
    }
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
                let asset_prices = instance
                    .get_asset_prices(mp.contract_address(), mp.asset_list())
                    .await?;
                mp.update_prices(
                    asset_prices,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
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
