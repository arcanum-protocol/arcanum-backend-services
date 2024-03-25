use crate::expiry::StdTimeExtractor;

use super::*;
use std::cmp::Ordering;

pub const POISON_TIME: u64 = 3;

fn are_option_maybe_expired_equal<V: PartialEq + Clone>(
    exp1: &Option<MayBeExpired<V, StdTimeExtractor>>,
    exp2: &Option<MayBeExpired<V, StdTimeExtractor>>,
) -> bool {
    match (exp1, exp2) {
        (Some(exp1), Some(exp2)) => {
            if let (Some(value1), Some(value2)) = (
                exp1.clone().not_older_than(POISON_TIME),
                exp2.clone().not_older_than(POISON_TIME),
            ) {
                value1 == value2
            } else {
                false
            }
        }
        (None, None) => true,
        _ => false,
    }
}

impl PartialEq for Multipool<StdTimeExtractor> {
    fn eq(&self, other: &Self) -> bool {
        let mut sorted_mp_assets = self.assets.clone();
        sorted_mp_assets.sort_by(compare_assets);

        let mut sorted_expected_assets = other.assets.clone();
        sorted_expected_assets.sort_by(compare_assets);

        self.contract_address == other.contract_address
            && sorted_mp_assets == sorted_expected_assets
            && are_option_maybe_expired_equal(&self.total_supply, &other.total_supply)
            && are_option_maybe_expired_equal(&self.total_shares, &other.total_shares)
    }
}

impl PartialEq for QuantityData {
    fn eq(&self, other: &Self) -> bool {
        self.quantity == other.quantity && self.cashback == other.cashback
    }
}

impl PartialEq for MultipoolAsset<StdTimeExtractor> {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
            && are_option_maybe_expired_equal(&self.price, &other.price)
            && self.quantity_slot == other.quantity_slot
            && self.share == other.share
    }
}

// TODO: create a mock extractor
#[derive(Clone)]
pub struct MultipoolMockBuilder(Multipool<StdTimeExtractor>);

impl MultipoolMockBuilder {
    pub fn new(contract_address: H160) -> Self {
        Self(Multipool::new(contract_address))
    }

    pub fn build(self) -> Multipool<StdTimeExtractor> {
        self.0
    }

    // insert empty assets
    pub fn insert_assets(mut self, addresses: Vec<H160>) -> Self {
        let mut assets: Vec<MultipoolAsset<StdTimeExtractor>> = Vec::new();
        for address in addresses {
            assets.push(MultipoolAsset {
                address,
                price: None,
                quantity_slot: None,
                share: None,
            })
        }
        self.0.assets = assets;
        self
    }

    // fill prices in multipool with certain value
    pub fn fill_updated_prices_with_value(self, addresses: Vec<H160>, value: U256) -> Self {
        let new_prices_info: Vec<(Address, Price)> = (0..addresses.len())
            .map(|i| (addresses[i], value))
            .collect();
        self.fill_updated_prices(new_prices_info)
    }

    // fill prices in multipool with diffrent values
    pub fn fill_updated_prices(mut self, values: Vec<(H160, U256)>) -> Self {
        self.0.update_prices(&values, false);
        self
    }

    // fill shares in multipool with certain value
    pub fn fill_updated_shares_with_value(self, addresses: Vec<H160>, value: U256) -> Self {
        let new_shares_info: Vec<(Address, Price)> = (0..addresses.len())
            .map(|i| (addresses[i], value))
            .collect();
        self.fill_updated_shares(new_shares_info)
    }

    // fill shares in multipool with diffrent values
    pub fn fill_updated_shares(mut self, values: Vec<(H160, U256)>) -> Self {
        self.0.update_shares(&values, false);
        self
    }

    // fill quantity in multipool with certain value
    pub fn fill_updated_quantities_with_value(
        self,
        addresses: Vec<H160>,
        value: U256,
        contract_address: Option<H160>,
    ) -> Self {
        let values: Vec<(H160, QuantityData)> = addresses
            .clone()
            .into_iter()
            .map(|address| (address, QuantityData::new(value, U256::zero())))
            .collect();
        self.fill_updated_quantities(values, contract_address)
    }

    // fill quantity in multipool with diffrent values
    pub fn fill_updated_quantities(
        mut self,
        values: Vec<(H160, QuantityData)>,
        contract_address: Option<H160>,
    ) -> Self {
        let mut values = values;
        if let Some(contract_address) = contract_address {
            let total_supply: U256 = values
                .iter()
                .fold(U256::zero(), |acc, (_, value)| acc + value.quantity);
            values.extend_from_slice(&[(
                contract_address,
                QuantityData::new(total_supply, U256::zero()),
            )]);
        }
        self.0.update_quantities(&values, false);
        self
    }

    pub fn set_price(mut self, address: H160, value: U256) -> Self {
        if let Some(asset) = self
            .0
            .assets
            .iter_mut()
            .find(|asset| asset.address == address)
        {
            asset.price = Some(MayBeExpired::new(value));
        } else {
            self.0.assets.push(MultipoolAsset {
                address,
                price: Some(MayBeExpired::new(value)),
                quantity_slot: Default::default(),
                share: Default::default(),
            });
        }
        self
    }

    pub fn set_share(mut self, address: H160, value: U256) -> Self {
        if let Some(asset) = self
            .0
            .assets
            .iter_mut()
            .find(|asset| asset.address == address)
        {
            println!("here");
            asset.share = Some(MayBeExpired::new(value));
        } else {
            self.0.assets.push(MultipoolAsset {
                address,
                price: Default::default(),
                quantity_slot: Default::default(),
                share: Some(MayBeExpired::new(value)),
            });
        }
        self
    }

    pub fn set_total_shares(mut self, value: U256) -> Self {
        self.0.total_shares = Some(MayBeExpired::new(value));
        self
    }

    pub fn set_quantity(mut self, address: H160, value: U256) -> Self {
        let quantity_data = QuantityData::new(value, U256::zero());
        if let Some(asset) = self
            .0
            .assets
            .iter_mut()
            .find(|asset| asset.address == address)
        {
            asset.quantity_slot = Some(MayBeExpired::new(quantity_data));
        } else if self.0.contract_address == address {
            self.0.total_supply = Some(MayBeExpired::new(value));
        } else {
            self.0.assets.push(MultipoolAsset {
                address,
                price: Default::default(),
                quantity_slot: Some(MayBeExpired::new(quantity_data)),
                share: Default::default(),
            });
        }
        self
    }
}

//fill multipool with similar values, but other way
pub fn multipool_fixture(
    contract_address: H160,
    addresses: Vec<H160>,
    value: U256,
) -> Multipool<StdTimeExtractor> {
    let mut assets: Vec<MultipoolAsset<StdTimeExtractor>> = Vec::new();
    let mut total_shares = U256::zero();
    let mut total_supply = U256::zero();
    for address in addresses {
        let share_number = value;
        let price_number = value;
        let quantity_data = QuantityData::new(value, U256::zero());
        let asset = MultipoolAsset {
            address,
            price: Some(MayBeExpired::new(price_number)),
            quantity_slot: Some(MayBeExpired::new(quantity_data.clone())),
            share: Some(MayBeExpired::new(share_number)),
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

pub fn read_method_fixture(contract_address: H160) -> Multipool<StdTimeExtractor> {
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
        QuantityData::new(U256::from(30) << 96, U256::zero()),
        QuantityData::new(U256::from(10) << 96, U256::zero()),
        QuantityData::new(U256::from(4) << 96, U256::zero()),
        QuantityData::new(U256::from(1441) << 96, U256::zero()),
        QuantityData::new(U256::from(440) << 96, U256::zero()),
    ];

    let new_prices_info: Vec<(H160, U256)> = ADDRESSES.iter().copied().zip(prices).collect();
    let new_shares_info: Vec<(H160, U256)> = ADDRESSES.iter().copied().zip(shares).collect();

    let mut new_quantity_info: Vec<(H160, QuantityData)> =
        ADDRESSES.iter().copied().zip(quantities).collect();
    new_quantity_info.extend_from_slice(&[(
        contract_address,
        QuantityData::new(U256::from(1893) << 96, U256::zero()),
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
