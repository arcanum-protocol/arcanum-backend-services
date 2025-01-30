pub mod errors;
pub mod read;

pub mod expiry;
#[cfg(test)]
pub mod tests;
pub mod write;

use alloy::primitives::aliases::U112;
use alloy::primitives::aliases::U96;
use alloy::primitives::ruint::aliases::U128;
use alloy::primitives::ruint::aliases::U256;
use alloy::primitives::Address;
use alloy::primitives::B256;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use expiry::EmptyTimeExtractor;

use std::ops::Shr;

use self::expiry::{MayBeExpired, Merge};
use errors::MultipoolErrors;
use errors::MultipoolErrors::*;
use errors::MultipoolOverflowErrors::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct Multipool {
    #[borsh(skip)]
    pub contract_address: Address,
    pub assets: Vec<MultipoolAsset>,
    #[borsh(skip)]
    pub total_supply: U256,

    pub deviation_increase_fee: u16,
    pub deviation_limit: u16,
    pub cashback_fee: u16,
    pub base_fee: u16,
    #[borsh(skip)]
    pub management_fee_receiver: Address,
    pub management_fee: u16,
    pub total_target_shares: u16,

    #[borsh(skip)]
    pub oracle_address: Address,
    #[borsh(skip)]
    pub initial_share_price: U96,
}

#[derive(Debug, Deserialize, Serialize, Clone, BorshSerialize, BorshDeserialize)]
pub struct MultipoolAsset {
    //#[borsh(deserialize_with = "")]
    //#[borsh(serialize_with = "")]
    #[borsh(skip)]
    pub address: Address,

    #[borsh(skip)]
    pub price_data: B256,
    #[borsh(skip)]
    pub price: Option<MayBeExpired<U256, EmptyTimeExtractor>>,

    #[borsh(skip)]
    pub quantity: U128,
    #[borsh(skip)]
    pub collected_cashbacks: U112,
    pub share: u16,
}

const X96: u64 = 96;
const X32: u64 = 32;

impl MultipoolAsset {
    fn new(address: Address) -> Self {
        Self {
            address,
            price_data: Default::default(),
            price: Default::default(),
            quantity: Default::default(),
            collected_cashbacks: Default::default(),
            share: Default::default(),
        }
    }

    fn quoted_quantity(&self) -> Result<MayBeExpired<U256, EmptyTimeExtractor>, MultipoolErrors> {
        self.price
            .clone()
            .ok_or(PriceMissing(self.address))?
            .map(|price| {
                U256::from(self.quantity)
                    .checked_mul(price)
                    .map(|v| v.shr(X96))
                    .ok_or(Overflow(QuotedQuantityOverflow))
            })
            .transpose()
    }
}

impl Multipool {
    pub fn new(contract_address: Address) -> Self {
        Self {
            contract_address,
            assets: Default::default(),
            total_supply: Default::default(),
            deviation_increase_fee: Default::default(),
            deviation_limit: Default::default(),
            cashback_fee: Default::default(),
            base_fee: Default::default(),
            management_fee_receiver: Default::default(),
            management_fee: Default::default(),
            total_target_shares: Default::default(),
            oracle_address: Default::default(),
            initial_share_price: Default::default(),
        }
    }
}
