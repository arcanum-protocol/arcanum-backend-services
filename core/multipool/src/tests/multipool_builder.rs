use alloy::primitives::{Address, U128, U256};
use multipool_types::Multipool::MultipoolEvents;

use multipool_types::expiry::StdTimeExtractor;

use std::cmp::Ordering;

use super::{EmptyTimeExtractor, MayBeExpired, Multipool, MultipoolAsset, ADDRESSES};

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
    addresses: Vec<(Address, U128, U256, u16)>,
    total_supply: U256,
) -> Multipool {
    let mut assets: Vec<MultipoolAsset> = Vec::new();
    let mut total_target_shares = Default::default();
    for (address, value, price, share) in addresses {
        let asset = MultipoolAsset {
            address,
            price: Some(MayBeExpired::with_time(price, 0)),
            quantity: value,
            price_data: Default::default(),
            collected_cashbacks: Default::default(),
            share,
        };
        total_target_shares += share;
        assets.push(asset)
    }
    Multipool {
        contract_address,
        assets,
        total_supply,
        total_target_shares,
        ..Default::default()
    }
}

pub fn read_method_fixture(contract_address: Address) -> Multipool {
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
