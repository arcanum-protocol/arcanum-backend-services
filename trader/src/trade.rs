use std::{fs, path::PathBuf, sync::Arc};

use ethers::prelude::*;

use anyhow::{anyhow, Result};
use multipool::Multipool;
use rpc_controller::RpcRobber;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::{
    contracts::multipool::{AssetArgs, MultipoolContract},
    execution::ForcePushArgs,
    uniswap::RETRIES,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoolInfo {
    pub fee: u32,
    pub address: Address,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AssetPools {
    pub address: Address,
    pub asset_symbol: String,
    pub base_is_token0: bool,
    pub pools: Vec<PoolInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Uniswap {
    pub eth_pools: Vec<AssetPools>,
}

impl Uniswap {
    pub fn try_from_file(path: PathBuf) -> Self {
        serde_yaml::from_slice(fs::read(path).expect("Config should exist").as_slice())
            .expect("Config should be valid")
    }

    pub fn get_pool_fee(&self, address: &Address) -> Result<AssetPools, String> {
        self.eth_pools
            .iter()
            .find(|a| a.address == *address)
            .map(ToOwned::to_owned)
            .ok_or("pool not found".into())
    }
}

pub struct TradingData {
    pub rpc: RpcRobber,
    pub multipool: Multipool,
    pub force_push: ForcePushArgs,
    pub uniswap: Arc<Uniswap>,
    pub weth: Address,
}

pub struct AssetsChoise<'a> {
    pub trading_data: &'a TradingData,
    pub asset1: Address,
    pub asset2: Address,
    pub deviation_bound: I256,
}

pub struct MultipoolChoise<'a> {
    pub trading_data_with_assets: &'a AssetsChoise<'a>,
    pub amount_in: U256,
    pub amount_out: U256,
    pub fee: I256,
}

pub struct SwapOutcome {
    pub estimated: U256,
    pub best_pool: Address,
    pub zero_for_one: bool,
    pub best_fee: u32,
}

pub struct UniswapChoise<'a> {
    pub trading_data: &'a MultipoolChoise<'a>,
    pub input: SwapOutcome,
    pub output: SwapOutcome,
}
