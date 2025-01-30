use alloy::primitives::{Address, U256};
use multipool_types::Multipool::MultipoolEvents;

use crate::expiry::StdTimeExtractor;

use std::cmp::Ordering;

use super::{MayBeExpired, Multipool, MultipoolAsset, ADDRESSES};

pub const POISON_TIME: u64 = 3;

impl PartialEq for Multipool {
    fn eq(&self, other: &Self) -> bool {
        let mut sorted_mp_assets = self.assets.clone();
        sorted_mp_assets.sort_by(compare_assets);

        let mut sorted_expected_assets = other.assets.clone();
        sorted_expected_assets.sort_by(compare_assets);

        self.contract_address == other.contract_address
            && sorted_mp_assets == sorted_expected_assets
            && self.total_supply == other.total_supply
            && self.total_target_shares == other.total_target_shares
    }
}

impl PartialEq for MultipoolAsset {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
            && self.price == other.price
            && self.quantity == other.quantity
            && self.share == other.share
    }
}

// TODO: create a mock extractor
#[derive(Clone)]
pub struct MultipoolMockBuilder(Multipool);

impl MultipoolMockBuilder {
    pub fn new(contract_address: Address) -> Self {
        Self(Multipool::new(contract_address))
    }

    pub fn build(self) -> Multipool {
        self.0
    }

    // insert empty assets
    pub fn insert_assets(mut self, addresses: Vec<Address>) -> Self {
        for address in addresses {
            self.0.assets.push(MultipoolAsset::new(address));
        }
        self
    }

    pub fn with_prices(mut self, values: Vec<(Address, U256)>, timestamp: u64) -> Self {
        self.0.update_prices(&values, timestamp);
        self
    }

    pub fn with_events(mut self, values: &[MultipoolEvents]) -> Self {
        self.0.apply_events(&values);
        self
    }

    pub fn with_price(mut self, address: Address, value: U256, timestamp: u64) -> Self {
        let p = Some(MayBeExpired::with_time(value, timestamp));
        if let Some(asset) = self
            .0
            .assets
            .iter_mut()
            .find(|asset| asset.address == address)
        {
            asset.price = p;
        } else {
            let mut asset = MultipoolAsset::new(address);
            asset.price = p;
            self.0.assets.push(asset);
        }
        self
    }
}

//fill multipool with similar values, but other way
pub fn multipool_fixture(
    contract_address: Address,
    addresses: Vec<(Address, U256, u16)>,
) -> Multipool {
    let mut assets: Vec<MultipoolAsset<StdTimeExtractor>> = Vec::new();
    let mut total_shares = U256::default();
    let mut total_supply = U256::default();
    for address in addresses {
        let share_number = value;
        let price_number = value;
        let asset = MultipoolAsset {
            address,
            price: Some(MayBeExpired::new(price_number)),
            quantity: value,
            collected_cashbacks: Default::default(),
            share: share,
        };
        total_supply += quantity_data.quantity;
        total_shares += share_number;
        assets.push(asset)
    }
    Multipool {
        fees: None,
        contract_address,
        assets,
        total_supply: Some(MayBeExpired::new(total_supply)),
        total_shares: Some(MayBeExpired::new(total_shares)),
    }
}

pub fn read_method_fixture(contract_address: Address) -> Multipool<StdTimeExtractor> {
    //target_shares will be 20% 5% 5% 30% 40% for 5 tokens
    let shares: Vec<U256> = vec![
        U256::from(200) << 96,
        U256::from(50) << 96,
        U256::from(50) << 96,
        U256::from(300) << 96,
        U256::from(400) << 96,
    ];
    let prices: Vec<U256> = vec![
        U256::from(15) << 96,
        U256::from(21) << 96,
        U256::from(54) << 96,
        U256::from(11) << 96,
        U256::from(191) << 96,
    ];
    let quantities: Vec<QuantityData> = vec![
        QuantityData::new(U256::from(30) << 96, U256::default()),
        QuantityData::new(U256::from(10) << 96, U256::default()),
        QuantityData::new(U256::from(4) << 96, U256::default()),
        QuantityData::new(U256::from(1441) << 96, U256::default()),
        QuantityData::new(U256::from(440) << 96, U256::default()),
    ];

    let new_prices_info: Vec<(Address, U256)> = ADDRESSES.iter().copied().zip(prices).collect();
    let new_shares_info: Vec<(Address, U256)> = ADDRESSES.iter().copied().zip(shares).collect();

    let mut new_quantity_info: Vec<(Address, QuantityData)> =
        ADDRESSES.iter().copied().zip(quantities).collect();
    new_quantity_info.extend_from_slice(&[(
        contract_address,
        QuantityData::new(U256::from(1893) << 96, U256::default()),
    )]);

    MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices(new_prices_info)
        .fill_updated_shares(new_shares_info)
        .fill_updated_quantities(new_quantity_info, None)
        .build()
}

pub fn compare_assets(
    asset1: &MultipoolAsset<StdTimeExtractor>,
    asset2: &MultipoolAsset<StdTimeExtractor>,
) -> Ordering {
    asset1.address.cmp(&asset2.address)
}
