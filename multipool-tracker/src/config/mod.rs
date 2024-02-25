use std::{collections::HashMap, fs};

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::{multipool::MultipoolId, rpc_controller::RpcParams, trader::analyzer::Uniswap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipoolConfig {
    pub contract_address: Address,
    pub initial_assets: Vec<Address>,
    pub price_fetcher: PriceFetcherConfig,
    pub event_fetcher: EventFetcherConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceFetcherConfig {
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFetcherConfig {
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub multipools: HashMap<MultipoolId, MultipoolConfig>,
    pub rpc: Vec<RpcParams>,
    pub chain_id: u128,
    pub poison_time: u64,
    pub uniswap: Option<Uniswap>,
}

impl BotConfig {
    pub fn from_file(config_path: &str) -> Self {
        serde_yaml::from_slice(
            fs::read(config_path)
                .expect("Config should exist")
                .as_slice(),
        )
        .expect("Config should be valid")
    }
}
