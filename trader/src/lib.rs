use std::{collections::HashMap, sync::Arc, time::Duration};

use alloy::primitives::{Bytes, I256};
use alloy::providers::Provider;
use clickhouse::Click;
use contracts::{trader::Trader::OraclePrice, WETH_ADDRESS};
use multipool::expiry::MayBeExpired;
use multipool_storage::hook::HookInitializer;
use tokio::{runtime::Handle, time::timeout};
use trade::{AssetsChoise, TradingData};

pub mod cashback;
pub mod clickhouse;
pub mod contracts;
pub mod execution;
pub mod strategies;
pub mod trade;
pub mod uniswap;

#[derive(Clone)]
pub struct TraderHook<P: Provider + Clone + 'static> {
    pub click: Arc<Click>,
    pub handle: Handle,
    pub rpc: P,
    pub task_timeout: Duration,
}

impl<P: Provider + Clone> HookInitializer for TraderHook<P> {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        getter: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        println!("Initialized hook");
        let hook_data = self.clone();
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let multipool = getter();
            let price = multipool
                .get_price(&multipool.contract_address)
                // no need to check if there is no asset, because we have multipool info
                .unwrap();
            let signed_price = some_sign_method(&price);
            let asset_list = multipool.asset_list();
            let contract_address = multipool.contract_address();
            let trading_data = Arc::new(TradingData {
                rpc: hook_data.rpc.clone(),
                multipool,
                silo_assets: HashMap::new(),
                oracle_price: OraclePrice {
                    contractAddress: contract_address,
                    timestamp: price.timestamp as u128,
                    sharePrice: price.value.to::<u128>(),
                    signature: signed_price,
                },
                weth: WETH_ADDRESS,
            });
            for asset1 in asset_list.iter() {
                for asset2 in asset_list.iter() {
                    let s = AssetsChoise {
                        trading_data: trading_data.clone(),
                        asset1: *asset1,
                        asset2: *asset2,
                        deviation_bound: I256::ZERO,
                    };
                    self.handle.spawn(async move {
                        let err = s.estimate_multipool().await;
                        match err {
                            Ok(v) => match v.estimate_uniswap().await {
                                Ok(v) => {
                                    let r =
                                        timeout(hook_data.task_timeout, v.execute()).await;
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
        }
    }
}

fn some_sign_method(
    _price: &MayBeExpired<alloy::primitives::Uint<256, 4>, multipool::expiry::EmptyTimeExtractor>,
) -> Bytes {
    Bytes::new()
}
