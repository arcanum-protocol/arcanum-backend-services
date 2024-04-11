pub mod errors;
pub mod read;

pub mod expiry;
#[cfg(test)]
pub mod tests;
pub mod write;

use ethers::types::Address;
use ethers::types::U64;
use expiry::TimeExtractor;
use primitive_types::U256;

use std::ops::Shr;

use self::expiry::{MayBeExpired, Merge};
use errors::MultipoolErrors;

use serde::{Deserialize, Serialize};

pub type MultipoolId = String;
pub type Price = U256;
pub type BlockNumber = U64;
pub type Quantity = U256;
pub type Share = U256;

#[derive(Debug, Deserialize, Serialize)]
pub struct Multipool<T: TimeExtractor> {
    pub contract_address: Address,
    pub assets: Vec<MultipoolAsset<T>>,
    pub total_supply: Option<MayBeExpired<Quantity, T>>,
    pub total_shares: Option<MayBeExpired<Share, T>>,
    pub fees: Option<MayBeExpired<MultipoolFees, T>>,
}

impl<T: TimeExtractor> Clone for Multipool<T> {
    fn clone(&self) -> Self {
        Self {
            contract_address: self.contract_address,
            assets: self.assets.clone(),
            total_supply: self.total_supply.clone(),
            total_shares: self.total_shares.clone(),
            fees: self.fees.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MultipoolFees {
    pub deviation_limit: U64,
    pub deviation_param: U64,
    pub depeg_base_fee: U64,
    pub base_fee: U64,
    pub developer_base_fee: U64,
    pub developer_address: Address,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultipoolAsset<T: TimeExtractor> {
    pub address: Address,
    pub price: Option<MayBeExpired<Price, T>>,
    pub quantity_slot: Option<MayBeExpired<QuantityData, T>>,
    pub share: Option<MayBeExpired<Share, T>>,
}

impl<T: TimeExtractor> Clone for MultipoolAsset<T> {
    fn clone(&self) -> Self {
        Self {
            address: self.address,
            price: self.price.clone(),
            quantity_slot: self.quantity_slot.clone(),
            share: self.share.clone(),
        }
    }
}

const X96: u128 = 96;
const X32: u128 = 32;

impl<T: TimeExtractor> MultipoolAsset<T> {
    fn quoted_quantity(&self) -> Result<MayBeExpired<Price, T>, MultipoolErrors> {
        let slot = self
            .quantity_slot
            .clone()
            .unwrap_or(MayBeExpired::new(QuantityData {
                quantity: U256::zero(),
                cashback: U256::zero(),
            }));
        let price = self
            .price
            .clone()
            .ok_or(MultipoolErrors::PriceMissing(self.address))?;
        (slot, price)
            .merge(|(slot, price)| -> Option<U256> {
                slot.quantity.checked_mul(price).map(|m| m.shr(X96))
            })
            .transpose()
            .ok_or(MultipoolErrors::QuotedQuantityMissing(self.address))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuantityData {
    pub quantity: Quantity,
    pub cashback: Quantity,
}

impl QuantityData {
    fn is_empty(&self) -> bool {
        self.quantity.is_zero() && self.cashback.is_zero()
    }
}

impl<T: TimeExtractor> Multipool<T> {
    pub fn new(contract_address: Address) -> Self {
        Self {
            contract_address,
            assets: Default::default(),
            total_supply: Default::default(),
            total_shares: Default::default(),
            fees: Default::default(),
        }
    }
}
