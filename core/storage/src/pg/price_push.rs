use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alloy::json_abi::Function;
use alloy::rpc::client::BatchRequest;
use alloy::{
    dyn_abi::DynSolValue,
    primitives::{Address, U256},
    providers::{ReqwestProvider, RootProvider},
    pubsub::PubSubFrontend,
    rpc::client::ReqwestClient,
    sol_types::SolInterface,
    transports::http::{Http, ReqwestTransport},
};

use sqlx::PgPool;

use crate::hook::HookInitializer;

#[derive(Clone)]
pub struct PricePush {
    pool: PgPool,
    delay: Duration,
    multicall_chunk_size: usize,
    multicall_address: Address,
    rpc: RootProvider<ReqwestTransport>,
}

pub async fn get_asset_prices(
    rpc: &RootProvider<ReqwestTransport>,
    mp: Address,
    assets: Vec<Address>,
) -> anyhow::Result<HashMap<Address, U256>> {
    use multipool_types::Multipool::{getPriceCall, MultipoolCalls};
    let mut mc = alloy_multicall::Multicall::new(rpc /* multicall address here */);

    for asset in assets {
        let call = MultipoolCalls::getPrice(getPriceCall { asset });
        mc.add_call(mp, call.selector().into(), &[asset.into()], false);
    }
    let result = mc.call().await?;
    Ok(assets
        .into_iter()
        .zip(result.into_iter())
        .map(|(address, price)| (address, price.unwrap().as_uint().unwrap().0))
        .collect())
}

impl HookInitializer for PricePush {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let instance = self.clone();
        vec![tokio::spawn(async move {
            loop {
                let mut mp = multipool();
                let asset_prices =
                    get_asset_prices(&instance.rpc, mp.contract_address(), mp.asset_list()).await?;
                mp.update_prices(
                    asset_prices,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
                let price = mp.get_price(&mp.contract_address());
            }
        })]
    }
}
