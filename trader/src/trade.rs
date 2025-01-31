use std::{collections::HashMap, sync::Arc};

use crate::contracts::trader::Trader::OraclePrice;
use alloy::{
    primitives::{Address, I256, U256},
    providers::RootProvider,
    transports::http::{Client, Http},
};
use multipool::Multipool;
use serde::{Deserialize, Serialize};

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

pub type HttpProvider = RootProvider<Http<Client>>;

pub struct TradingData {
    pub rpc: RootProvider<Http<Client>>,
    pub multipool: Multipool,
    pub oracle_price: OraclePrice,
    pub silo_assets: HashMap<Address, (Address, Address)>,
    pub weth: Address,
}

pub struct AssetsChoise {
    pub trading_data: Arc<TradingData>,
    pub asset1: Address,
    pub asset2: Address,
    pub deviation_bound: I256,
}

pub struct WrapperCall {
    pub wrapper: Address,
    pub data: Vec<u8>,
}

pub struct MultipoolChoise {
    pub trading_data_with_assets: AssetsChoise,

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

pub struct UniswapChoise {
    pub trading_data: MultipoolChoise,
    pub input: SwapOutcome,
    pub output: SwapOutcome,
}
