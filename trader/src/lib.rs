use std::{sync::Arc, time::Duration};

use execution::ForcePushArgs;
use multipool_storage::MultipoolStorageHook;
use rpc_controller::RpcRobber;
use tokio::runtime::Handle;
use trade::{AssetsChoise, TradingData, Uniswap};

use ethers::prelude::*;

pub mod contracts;
pub mod execution;
pub mod strategies;
pub mod trade;
pub mod uniswap;

#[derive(Clone)]
pub struct TraderHook {
    pub cache: Arc<multipool_cache::cache::CachedMultipoolData>,
    pub handle: Handle,
    pub uniswap: Arc<Uniswap>,
    pub rpc: RpcRobber,
    pub weth: Address,
}

impl MultipoolStorageHook for TraderHook {
    fn new_pool(
        &self,
        pool: std::sync::Arc<
            tokio::sync::RwLock<multipool_storage::multipool_with_meta::MultipoolWithMeta>,
        >,
    ) {
        let hook_data = self.clone();
        self.handle.spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;
                let multipool = { pool.read().await.multipool.clone() };
                println!("{}", multipool.contract_address());
                if !multipool
                    .contract_address()
                    .eq(&"0x4810E5A7741ea5fdbb658eDA632ddfAc3b19e3c6"
                        .parse()
                        .unwrap())
                {
                    continue;
                }
                println!("{}", multipool.contract_address());
                let signed_price = match hook_data
                    .cache
                    .get_signed_price(&multipool.contract_address())
                {
                    Some(v) => v,
                    None => continue,
                };
                let asset_list = multipool.asset_list();
                let contract_address = multipool.contract_address();
                let trading_data = TradingData {
                    rpc: hook_data.rpc.clone(),
                    multipool,
                    force_push: ForcePushArgs {
                        contract_address,
                        timestamp: signed_price.timestamp.parse().unwrap(),
                        share_price: signed_price.share_price.parse().unwrap(),
                        signatures: vec![signed_price.signature.parse().unwrap()],
                    },
                    weth: hook_data.weth,
                    uniswap: hook_data.uniswap.clone(),
                };
                for asset1 in asset_list.iter() {
                    for asset2 in asset_list.iter() {
                        let s = AssetsChoise {
                            trading_data: &trading_data,
                            asset1: *asset1,
                            asset2: *asset2,
                            deviation_bound: 0.into(),
                        };
                        let err = s.estimate_multipool().await;
                        match err {
                            Ok(v) => match v.estimate_uniswap().await {
                                Ok(v) => {
                                    let r = v.execute().await;
                                    println!("{r:?}");
                                }
                                Err(e) => {
                                    println!("{e:?}");
                                    continue;
                                }
                            },
                            Err(_e) => {
                                //println!("{e:?}");
                                continue;
                            }
                        }
                    }
                }
            }
        });
    }
}
