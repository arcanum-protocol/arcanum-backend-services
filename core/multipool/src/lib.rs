pub mod errors;
pub mod read;

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

use std::ops::Shr;

use errors::MultipoolErrors;
use errors::MultipoolErrors::*;
use errors::MultipoolOverflowErrors::*;
use multipool_types::expiry::{EmptyTimeExtractor, MayBeExpired, Merge};
use serde::Serialize;

use multipool_types::borsh_methods::{deserialize, serialize};

#[derive(Debug, Clone, Default, BorshSerialize, BorshDeserialize, Serialize)]
pub struct Multipool {
    #[borsh(
        deserialize_with = "deserialize::address",
        serialize_with = "serialize::address"
    )]
    pub contract_address: Address,
    pub chain_id: u64,
    pub assets: Vec<MultipoolAsset>,
    #[borsh(
        deserialize_with = "deserialize::u256",
        serialize_with = "serialize::u256"
    )]
    pub total_supply: U256,

    #[borsh(
        deserialize_with = "deserialize::address",
        serialize_with = "serialize::address"
    )]
    pub strategy_manager: Address,
    #[borsh(
        deserialize_with = "deserialize::address",
        serialize_with = "serialize::address"
    )]
    pub owner: Address,

    pub deviation_increase_fee: u16,
    pub deviation_limit: u16,
    pub cashback_fee: u16,
    pub base_fee: u16,
    #[borsh(
        deserialize_with = "deserialize::address",
        serialize_with = "serialize::address"
    )]
    pub management_fee_receiver: Address,
    pub management_fee: u16,
    pub total_target_shares: u16,

    #[borsh(
        deserialize_with = "deserialize::address",
        serialize_with = "serialize::address"
    )]
    pub oracle_address: Address,
    #[borsh(
        deserialize_with = "deserialize::u96",
        serialize_with = "serialize::u96"
    )]
    pub initial_share_price: U96,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize)]
pub struct MultipoolAsset {
    #[borsh(
        deserialize_with = "deserialize::address",
        serialize_with = "serialize::address"
    )]
    pub address: Address,

    #[borsh(
        deserialize_with = "deserialize::b256",
        serialize_with = "serialize::b256"
    )]
    pub price_data: B256,

    pub price: Option<MayBeExpired<U256, EmptyTimeExtractor>>,

    #[borsh(
        deserialize_with = "deserialize::u128",
        serialize_with = "serialize::u128"
    )]
    pub quantity: U128,

    #[borsh(
        deserialize_with = "deserialize::u112",
        serialize_with = "serialize::u112"
    )]
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
    pub fn new(contract_address: Address, chain_id: u64) -> Self {
        Self {
            contract_address,
            chain_id,
            ..Default::default()
        }
    }
}
