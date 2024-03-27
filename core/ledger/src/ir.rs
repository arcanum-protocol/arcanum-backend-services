use std::time::{SystemTime, UNIX_EPOCH};

use ethers::prelude::*;

use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Time {
    pub timestamp: u64,
    pub block: U64,
}

impl Time {
    pub fn new(block: U64) -> Self {
        Self {
            block,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Shold be always after epoch start")
                .as_secs(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultipoolFactoryIR {
    pub factory_block: u64,
    pub factory_address: Address,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct MultipoolStorageIR {
    pub pools: Vec<MultipoolIR>,
    pub factories: Vec<MultipoolFactoryIR>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultipoolIR {
    pub contract_address: Address,
    pub assets: Vec<MultipoolAssetIR>,
    pub total_supply: Option<U256>,
    pub total_shares: Option<U256>,
    pub share_time: Time,
    pub quantity_time: Time,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultipoolAssetIR {
    pub address: Address,
    pub quantity_slot: Option<QuantityDataIr>,
    pub share: Option<U256>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuantityDataIr {
    pub quantity: U256,
    pub cashback: U256,
}

impl MultipoolStorageIR {
    pub fn try_pack(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(Into::into)
    }

    pub fn try_unpack(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(Into::into)
    }
}
