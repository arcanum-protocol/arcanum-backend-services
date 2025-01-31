use crate::contracts::multipool::MultipoolContract;
use crate::contracts::MULTICALL_ADDRESS;
use crate::trade::HttpProvider;
use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};
use alloy_multicall::Multicall;
use anyhow::Result;
use std::collections::HashMap;

// SHOULD ACCEPT assets AS LIST OF ALL TOKENS + MP ITSELF
pub async fn get_asset_prices(
    rpc: &HttpProvider,
    mp: Address,
    assets: Vec<Address>,
) -> Result<HashMap<Address, U256>> {
    let mut mc = Multicall::new(rpc, MULTICALL_ADDRESS);
    let f = MultipoolContract::abi::functions();
    let get_price_func = &f.get("getPrice").unwrap()[0];
    for a in assets.iter() {
        mc.add_call(mp, &get_price_func, &[DynSolValue::Address(*a)], true);
    }
    let result = mc.call().await?;
    Ok(assets
        .into_iter()
        .zip(result.into_iter())
        .map(|(address, price)| (address, price.unwrap().as_uint().unwrap().0))
        .collect())
}
