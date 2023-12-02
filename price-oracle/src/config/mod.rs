use std::{collections::HashMap, fs};

use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::multipool_storage::{MultipoolFetchParams, MultipoolId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipoolConfig {
    pub fetch_params: MultipoolFetchParams,
    pub contract_address: Address,
    pub assets: Vec<Address>,
    pub provider_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub multipools: HashMap<MultipoolId, MultipoolConfig>,
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
