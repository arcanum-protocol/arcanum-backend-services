use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, MULTICALL3_ADDRESS};
use itertools::Itertools;
use std::collections::HashMap;

pub async fn get_asset_prices<P: Provider + Clone + 'static>(
    mp: Address,
    assets: Vec<Address>,
    chunk_size: usize,
    provider: &P,
) -> anyhow::Result<HashMap<Address, U256>> {
    let multipool_functions = multipool_types::Multipool::abi::functions();
    let get_price_func = &multipool_functions.get("getPrice").unwrap()[0];

    let mut prices = Vec::new();
    let chunked_assets = assets
        .iter()
        .chunks(chunk_size)
        .into_iter()
        .map(|chunk| chunk.into_iter().collect_vec())
        .collect_vec();
    for chunk in chunked_assets {
        let mut mc = alloy_multicall::Multicall::new(&provider, MULTICALL3_ADDRESS);
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
