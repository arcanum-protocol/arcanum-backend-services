use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::cache::cache::Cache;
use crate::cache::multipool::Multipool;
use crate::clickhouse::Click;
use crate::contracts::{trader::Trader::OraclePrice, WETH_ADDRESS};
use crate::trade::{AssetsChoise, TradingData};
use alloy::primitives::{Address, Bytes, I256, U256};
use alloy::providers::Provider;
use anyhow::Result;
use multipool_storage::hook::HookInitializer;
use multipool_types::expiry::MayBeExpired;
use tokio::{runtime::Handle, time::timeout};

#[derive(Clone)]
pub struct TraderHook<P: Provider + Clone + 'static> {
    pub click: Arc<Click>,
    pub handle: Handle,
    pub rpc: P,
    pub task_timeout: Duration,
}

pub async fn process_pool<P: Provider + Sync + Send + 'static>(
    cache: Arc<Cache<P>>,
    click: Arc<Click>,
    multipool: Multipool,
    task_timeout: Duration,
) -> Result<()> {
    println!("Initialized hook");
        tokio::time::sleep(Duration::from_secs(3)).await;
        let price = multipool.cap;
        let signed_price = some_sign_method(&price);
        let contract_address = multipool.address;
        let asset_list = &multipool.assets_addresses;
        let trading_data = Arc::new(TradingData {
            rpc: cache.provider.clone(),
            multipool: multipool.clone(),
            silo_assets: HashMap::new(),
            oracle_price: OraclePrice {
                contractAddress: contract_address,
                // timestamp: price.time() as u128,
                timestamp: 0,
                sharePrice: price.to::<u128>(),
                signature: signed_price,
            },
            weth: WETH_ADDRESS,
        });
        for asset1 in asset_list.iter() {
            for asset2 in asset_list.iter() {
                if asset1 == asset2 {continue;}
                let s = AssetsChoise {
                    trading_data: trading_data.clone(),
                    asset1: *asset1,
                    asset2: *asset2,
                    deviation_bound: I256::ZERO,
                };
                let click = click.clone();
                tokio::spawn(async move {
                    let err = s.estimate_multipool().await;
                    match err {
                        Ok(v) => match v.estimate_uniswap().await {
                            Ok(v) => {
                                let r = timeout(task_timeout, v.execute(click)).await;
                                println!("Send trade result: {r:?}");
                            }
                            Err(e) => {
                                println!("Estimate Uniswap error: {e:?}");
                            }
                        },
                        Err(_e) => {}
                    }
                });
            }
        }
        Ok(())
    }

fn some_sign_method(_price: &U256) -> Bytes {
    Bytes::new()
}
