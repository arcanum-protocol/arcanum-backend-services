use std::{collections::HashMap, fs};

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::{
    multipool_storage::{MultipoolFetchParams, MultipoolId},
    trader::analyzer::Uniswap,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipoolConfig {
    pub fetch_params: MultipoolFetchParams,
    pub contract_address: Address,
    pub assets: Vec<Address>,
    pub provider_url: String,
    pub chain_id: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub multipools: HashMap<MultipoolId, MultipoolConfig>,
    pub poison_time: u64,
    pub uniswap: Uniswap,
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
