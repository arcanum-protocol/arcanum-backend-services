use super::cases::ADDRESSES;
use super::*;
use ethers::prelude::*;
use std::ops::Deref;

const POISON_TIME: u64 = 5;

fn are_option_maybe_expired_equal<V: PartialEq + Clone>(
    exp1: &Option<MayBeExpired<V>>,
    exp2: &Option<MayBeExpired<V>>,
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

impl PartialEq for Multipool {
    fn eq(&self, other: &Self) -> bool {
        self.contract_address == other.contract_address
            && self.assets == other.assets
            && are_option_maybe_expired_equal(&self.total_supply, &other.total_supply)
            && are_option_maybe_expired_equal(&self.total_shares, &other.total_shares)
    }
}

impl PartialEq for QuantityData {
    fn eq(&self, other: &Self) -> bool {
        self.quantity == other.quantity && self.cashback == other.cashback
    }
}

impl PartialEq for MultipoolAsset {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
            && are_option_maybe_expired_equal(&self.price, &other.price)
            && self.quantity_slot == other.quantity_slot
            && self.share == other.share
    }
}

#[derive(Clone)]
pub struct MultipoolMockBuilder(Multipool);

impl Deref for MultipoolMockBuilder {
    type Target = Multipool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MultipoolMockBuilder {
    pub fn new(contract_address: H160) -> Self {
        Self(Multipool::new(contract_address))
    }

    pub fn build(self) -> Multipool {
        self.0
    }

    // insert empty assets
    pub fn insert_assets(mut self, addresses: Vec<H160>) -> Self {
        let mut assets: Vec<MultipoolAsset> = Vec::new();
        for i in 0..addresses.len() {
            assets.push(MultipoolAsset {
                address: addresses[i as usize],
                price: None,
                quantity_slot: None,
                share: None,
            })
        }
        self.0.assets = assets;
        self
    }

    //fill multipool with similar values, but other way
    pub fn multipool_fixture(
        self,
        contract_address: H160,
        addresses: Vec<H160>,
        value: U256,
    ) -> Multipool {
        let mut assets: Vec<MultipoolAsset> = Vec::new();
        let mut total_shares = U256::zero();
        let mut total_supply = U256::zero();
        for i in 0..addresses.len() {
            let share_number = value;
            let price_number = value;
            let quantity_data = QuantityData {
                quantity: value,
                cashback: U256::zero(),
            };
            let asset = MultipoolAsset {
                address: addresses[i as usize],
                price: Some(MayBeExpired::new(price_number)),
                quantity_slot: Some(MayBeExpired::new(quantity_data.clone())),
                share: Some(MayBeExpired::new(share_number)),
            };
            total_supply += quantity_data.quantity;
            total_shares += share_number;
            assets.push(asset)
        }
        Multipool {
            contract_address,
            assets,
            total_supply: Some(MayBeExpired::new(total_supply)),
            total_shares: Some(MayBeExpired::new(total_shares)),
        }
    }

    // fill prices in multipool with certain value
    pub fn fill_updated_prices_with_value(mut self, addresses: Vec<H160>, value: U256) -> Self {
        let new_prices_info: Vec<(Address, Price)> = (0..addresses.len())
            .map(|i| (addresses[i as usize], value))
            .collect();
        self.0.update_prices(&new_prices_info, false);
        self
    }

    // fill prices in multipool with diffrent values
    pub fn fill_updated_prices(mut self, values: Vec<(H160, U256)>) -> Self {
        self.0.update_prices(&values, false);
        self
    }

    // fill shares in multipool with certain value
    pub fn fill_updated_shares_with_value(mut self, addresses: Vec<H160>, value: U256) -> Self {
        let new_shares_info: Vec<(Address, Price)> = (0..addresses.len())
            .map(|i| (addresses[i as usize], value))
            .collect();
        self.0.update_shares(&new_shares_info, false);
        self
    }

    // fill shares in multipool with diffrent values
    pub fn fill_updated_shares(mut self, values: Vec<(H160, U256)>) -> Self {
        self.0.update_shares(&values, false);
        self
    }

    // fill quantity in multipool with certain value
    pub fn fill_updated_quantities_with_value(
        mut self,
        addresses: Vec<H160>,
        value: U256,
        contract_address: Option<H160>,
    ) -> Self {
        let mut values: Vec<(H160, QuantityData)> = addresses
            .clone()
            .into_iter()
            .map(|address| {
                (
                    address,
                    QuantityData {
                        quantity: value,
                        cashback: U256::zero(),
                    },
                )
            })
            .collect();
        if let Some(contract_address) = contract_address {
            values.extend_from_slice(&[(
                contract_address,
                QuantityData {
                    quantity: U256::from(addresses.len()) * value,
                    cashback: U256::zero(),
                },
            )])
        }
        self.0.update_quantities(&values, false);
        self
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
                QuantityData {
                    quantity: total_supply,
                    cashback: U256::zero(),
                },
            )]);
        }
        self.0.update_quantities(&values, false);
        self
    }
}

pub fn read_method_fixture(contract_address: H160) -> Multipool {
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
        QuantityData {
            quantity: U256::from(30) << 96,
            cashback: U256::zero(),
        },
        QuantityData {
            quantity: U256::from(8) << 96,
            cashback: U256::zero(),
        },
        QuantityData {
            quantity: U256::from(4) << 96,
            cashback: U256::zero(),
        },
        QuantityData {
            quantity: U256::from(1411) << 96,
            cashback: U256::zero(),
        },
        QuantityData {
            quantity: U256::from(440) << 96,
            cashback: U256::zero(),
        },
    ];

    let new_prices_info: Vec<(H160, U256)> = ADDRESSES
        .to_vec()
        .into_iter()
        .zip(prices.into_iter())
        .collect();

    let new_shares_info: Vec<(H160, U256)> = ADDRESSES
        .to_vec()
        .into_iter()
        .zip(shares.into_iter())
        .collect();

    let new_quantity_info: Vec<(H160, QuantityData)> = ADDRESSES
        .to_vec()
        .into_iter()
        .zip(quantities.into_iter())
        .collect();

    MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices(new_prices_info)
        .fill_updated_shares(new_shares_info)
        .fill_updated_quantities(new_quantity_info, None)
        .build()
}
