pub mod errors;
pub mod read;

pub mod expiry;
#[cfg(test)]
pub mod tests;
pub mod write;

use ethers::types::Address;
use ethers::types::U64;
use primitive_types::U256;
use std::collections::BTreeMap;

use std::ops::Shr;
use std::sync::Arc;
use tokio::sync::RwLock;

use self::expiry::{MayBeExpired, Merge};
use errors::MultipoolErrors;

pub type MultipoolId = String;
pub type Price = U256;
pub type BlockNumber = U64;
pub type Quantity = U256;
pub type Share = U256;

#[derive(Clone, Debug)]
pub struct MultipoolStorage {
    pub inner: BTreeMap<MultipoolId, Arc<RwLock<Multipool>>>,
}

impl std::ops::Deref for MultipoolStorage {
    type Target = BTreeMap<MultipoolId, Arc<RwLock<Multipool>>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone)]
pub struct Multipool {
    contract_address: Address,
    assets: Vec<MultipoolAsset>,
    total_supply: Option<MayBeExpired<Quantity>>,
    total_shares: Option<MayBeExpired<Share>>,
}

#[derive(Debug, Clone)]
pub struct MultipoolAsset {
    address: Address,
    price: Option<MayBeExpired<Price>>,
    quantity_slot: Option<MayBeExpired<QuantityData>>,
    share: Option<MayBeExpired<Share>>,
}

const X96: u128 = 96;
const X32: u128 = 32;

impl MultipoolAsset {
    fn quoted_quantity(&self) -> Result<MayBeExpired<Price>, MultipoolErrors> {
        let slot = self
            .quantity_slot
            .clone()
            .ok_or(MultipoolErrors::QuantitySlotMissing(self.address))?;
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

#[derive(Debug, Clone)]
pub struct QuantityData {
    pub quantity: Quantity,
    pub cashback: Quantity,
}

impl QuantityData {
    fn is_empty(&self) -> bool {
        self.quantity.is_zero() && self.cashback.is_zero()
    }
}

impl Multipool {
    pub fn new(contract_address: Address) -> Self {
        Self {
            contract_address,
            assets: Default::default(),
            total_supply: Default::default(),
            total_shares: Default::default(),
        }
    }
}
