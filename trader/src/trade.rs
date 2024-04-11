use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use ethers::prelude::*;

use anyhow::Result;
use multipool::{expiry::StdTimeExtractor, Multipool};
use rpc_controller::RpcRobber;
use serde::{Deserialize, Serialize};

use crate::execution::ForcePushArgs;

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
    pub silo_assets: HashMap<Address, (Address, Address)>,
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
    pub multipool: Multipool<StdTimeExtractor>,
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

pub struct WrapperCall {
    pub wrapper: Address,
    pub data: Vec<u8>,
}

pub struct MultipoolChoise<'a> {
    pub trading_data_with_assets: &'a AssetsChoise<'a>,

    pub unwrapped_amount_in: U256,
    pub unwrapped_amount_out: U256,

    pub multipool_amount_in: U256,
    pub multipool_amount_out: U256,

    pub swap_asset_in: Address,
    pub swap_asset_out: Address,

    pub wrap_call: WrapperCall,
    pub unwrap_call: WrapperCall,

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
