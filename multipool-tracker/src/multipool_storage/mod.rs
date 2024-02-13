pub mod expiry;
pub mod read;
pub mod write;

use ethers::types::Address;
use ethers::types::U64;
use primitive_types::U256;
use std::collections::BTreeMap;

use std::ops::Shr;
use std::sync::Arc;
use std::sync::RwLock;

use self::expiry::MayBeExpired;

pub type MultipoolId = String;
pub type Price = U256;
pub type BlockNumber = U64;
pub type Quantity = U256;
pub type Share = U256;

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
    fn quoted_quantity(&self) -> Option<MayBeExpired<Price>> {
        merge(
            self.quantity_slot.clone(),
            self.price.clone(),
            |slot, price| {
                slot.merge(price, |slot, price| -> Option<U256> {
                    slot.quantity.checked_mul(price).map(|m| m.shr(X96))
                })
                .transpose()
            },
        )
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

fn merge<T1, T2, T3, F: FnOnce(T1, T2) -> Option<T3>>(
    first: Option<T1>,
    second: Option<T2>,
    merger: F,
) -> Option<T3> {
    merger(first?, second?)
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
