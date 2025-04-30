use alloy::{dyn_abi::DynSolValue, primitives::{Address, U256}, providers::{Provider, MULTICALL3_ADDRESS}};
use anyhow::Result;
use alloy_multicall::Multicall;
use anyhow::Context;

use crate::contracts::multipool::MultipoolContract;

pub struct Multipool {
    pub address: Address,
    pub assets: Vec<MpAsset>,
    pub share_sum: U256,
    pub mp_price: U256,
    pub total_supply: U256
}

pub struct MpAsset {
    pub address: Address,
    pub share: U256,
    pub quantity: U256,
    pub price: U256,
    pub target_share: U256
}

impl Multipool {
    pub async fn from_rpc<T: Provider + Sync + Send + 'static>(rpc: &T, address: Address, total_supply: U256) -> Result<Self> {
        let fns = MultipoolContract::abi::functions();
        let get_used = &fns.get("getUsedAssets").context("Cannot find getUsedAssets fn")?[0];
        let get_context = &fns.get("getContext").context("Cannot find getContext fn")?[0];
        let mut mc = Multicall::new(rpc, MULTICALL3_ADDRESS);
        mc.add_call(address, &get_used, &[DynSolValue::Uint(U256::MAX, 256), DynSolValue::Uint(U256::MIN, 256)], true);
        mc.add_call(address, &get_context, &[DynSolValue::Uint(U256::MAX, 256), DynSolValue::Uint(U256::MIN, 256)], true);
        todo!()
    } 
}