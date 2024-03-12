use super::*;
use ethers::prelude::*;
use lazy_static::lazy_static;
use pretty_assertions::{assert_eq, assert_ne};
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

lazy_static! {
    static ref ADDRESSES: [Address; 5] = [
        H160::from_low_u64_le(1),
        H160::from_low_u64_le(2),
        H160::from_low_u64_le(3),
        H160::from_low_u64_le(4),
        H160::from_low_u64_le(5),
    ];
}

fn multipool_fixture(contract_address: H160, addresses: Vec<H160>, value: U256) -> Multipool {
    let mut assets: Vec<MultipoolAsset> = Vec::new();
    let mut total_shares = U256::zero();
    let mut total_supply = U256::zero();
    for i in 0..5 {
        let share_number = value;
        let price_number = value;
        let quantity_data = QuantityData {
            quantity: value,
            cashback: value,
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

#[derive(Clone)]
struct MultipoolMockBuilder(Multipool);

impl Deref for MultipoolMockBuilder {
    type Target = Multipool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MultipoolMockBuilder {
    fn new(contract_address: H160) -> Self {
        Self(Multipool::new(contract_address))
    }

    fn build(self) -> Multipool {
        self.0
    }

    // insert empty assets
    fn insert_assets(mut self, addresses: Vec<H160>) -> Self {
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

    // fill prices in multipool with certain value
    fn fill_updated_prices(mut self, addresses: Vec<H160>, value: U256) -> Self {
        let new_prices_info: Vec<(Address, Price)> = (0..addresses.len())
            .map(|i| (addresses[i as usize], value))
            .collect();
        self.0.update_prices(&new_prices_info, false);
        self
    }

    // fill prices in multipool with certain value
    fn fill_updated_shares(mut self, addresses: Vec<H160>, value: U256) -> Self {
        let new_shares_info: Vec<(Address, Price)> = (0..addresses.len())
            .map(|i| (addresses[i as usize], value))
            .collect();
        self.0.update_shares(&new_shares_info, false);
        self
    }

    fn fill_updated_quantities(
        mut self,
        addresses: Vec<H160>,
        contract_address: H160,
        value: U256,
        update_total_supply: bool,
    ) -> Self {
        let mut total_supply = U256::zero();
        let mut new_quantity_info: Vec<(Address, QuantityData)> = (0..addresses.len())
            .map(|i| {
                if update_total_supply {
                    total_supply += value;
                }
                (
                    addresses[i as usize],
                    QuantityData {
                        cashback: value,
                        quantity: value,
                    },
                )
            })
            .collect();
        if update_total_supply {
            new_quantity_info.extend_from_slice(&[(
                contract_address,
                QuantityData {
                    quantity: total_supply,
                    cashback: U256::zero(),
                },
            )]);
        }
        self.0.update_quantities(&new_quantity_info, false);
        self
    }
}

#[test]
fn check_base() {
    let contract_address: H160 = Address::random();
    let expected = multipool_fixture(contract_address, ADDRESSES.to_vec(), U256::from(10) << 96);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices(ADDRESSES.to_vec(), U256::from(10) << 96)
        .fill_updated_shares(ADDRESSES.to_vec(), U256::from(10) << 96)
        .fill_updated_quantities(
            ADDRESSES.to_vec(),
            contract_address,
            U256::from(10) << 96,
            true,
        )
        .build();
    assert_eq!(expected, multipool)
}

#[test]
//build new instance of multipool with read methods
//and also check others read method robustness
fn check_read_methods() {
    let contract_address = H160::from_low_u64_be(0x10);
    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices(ADDRESSES.to_vec(), U256::from(10))
        .fill_updated_shares(ADDRESSES.to_vec(), U256::from(10))
        .fill_updated_quantities(ADDRESSES.to_vec(), contract_address, U256::from(10), true)
        .build();
    let mut multipool = MultipoolMockBuilder::new(contract_address).build();

    // fill multipool with values
    let addresses = expected.asset_list();
    let mut total_shares: MayBeExpired<U256> = MayBeExpired::new(U256::zero());
    let mut total_supply: MayBeExpired<U256> = MayBeExpired::new(U256::zero());
    let mut assets: Vec<MultipoolAsset> = vec![];
    for address in addresses.iter() {
        let price = Some(expected.get_price(address).unwrap());
        let expected_asset = expected.asset(address).unwrap();
        let share = expected_asset.share;
        let quantity_slot = expected_asset.quantity_slot;
        let asset = MultipoolAsset {
            address: *address,
            price,
            share: share.clone(),
            quantity_slot: quantity_slot.clone(),
        };
        assets.push(asset);

        total_shares = (total_shares, share.unwrap()).merge(|(ts, s)| ts + s);
        total_supply = (
            total_supply,
            MayBeExpired::new(quantity_slot.unwrap().any_age().quantity),
        )
            .merge(|(ts, q)| ts + q);
    }
    multipool.assets = assets;
    multipool.total_shares = Some(total_shares);
    multipool.total_supply = Some(total_supply);
    assert_eq!(expected, multipool)
}

//#[test]
//fn check_deviation_correctness() {
//    let contract_address = H160::from_low_u64_be(0x10);
//    let multipool = MultipoolMockBuilder::new(contract_address)
//        .insert_assets(ADDRESSES.to_vec())
//        .fill_updated_prices(ADDRESSES.to_vec(), U256::from(10))
//        .fill_updated_shares(ADDRESSES.to_vec(), U256::from(10))
//        .fill_updated_quantities(ADDRESSES.to_vec(), contract_address, U256::from(10), true)
//        .build();
//    let deviation = multipool.deviation(&ADDRESSES[2].clone());
//    //multipool.quantity_to_deviation(&ADDRESSES[2], I256::new(pow_x96));
//    //for address in multipool.asset_list().into_iter() {
//    //}
//}

//NOTE ERRORS

//MULTIPOOL ASSET ERRORS
#[test]
fn check_asset_missing() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address);
    let expected_error = MultipoolErrors::AssetMissing(contract_address);
    assert_eq!(
        Err(expected_error),
        multipool.build().asset(&contract_address)
    );
}

//MULTIPOOL CAP ERRORS

#[test]
fn check_cap_fields_missing() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .build();

    // check quantity slot missing
    let expected_error = MultipoolErrors::QuantitySlotMissing(ADDRESSES[0].clone());
    assert_eq!(Err(expected_error), multipool.cap());

    // modify data for next error
    multipool.assets.iter_mut().for_each(|asset| {
        asset.quantity_slot = Some(MayBeExpired::new(QuantityData {
            cashback: U256::from(10),
            quantity: U256::from(10),
        }))
    });

    // check price missing
    let expected_error = MultipoolErrors::PriceMissing(ADDRESSES[0].clone());
    assert_eq!(Err(expected_error), multipool.cap());
}

#[test]
fn check_cap_overflow() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities(ADDRESSES.to_vec(), contract_address, U256::MAX, false)
        .fill_updated_prices(ADDRESSES.to_vec(), U256::from(2))
        .build();

    println!("{:?}", multipool);

    let expected_error =
        MultipoolErrors::Overflow(errors::MultipoolOverflowErrors::QuotedQuantityOverflow);
    assert_eq!(Err(expected_error), multipool.cap());
}

// MULTIPOOL TARGET SHARE ERROR
#[test]
fn check_target_share_share_missing() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities(ADDRESSES.to_vec(), contract_address, U256::from(10), true)
        .build();

    let expected_error = MultipoolErrors::ShareMissing(ADDRESSES[1].clone());
    assert_eq!(
        Err(expected_error),
        multipool.clone().target_share(&ADDRESSES[1].clone())
    );
}

// MULTIPOOL QUANTITY TO DEVIATION ERRORS
#[test]
fn check_quantity_to_deviation_overflow() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_shares(ADDRESSES.to_vec(), U256::from(10))
        .fill_updated_quantities(ADDRESSES.to_vec(), contract_address, U256::from(10), true)
        .fill_updated_prices(ADDRESSES.to_vec(), U256::from(10))
        .build();

    let expected_error =
        MultipoolErrors::Overflow(errors::MultipoolOverflowErrors::TargetDeviationOverflow);

    assert_eq!(
        Err(expected_error),
        multipool.quantity_to_deviation(&ADDRESSES[1].clone(), I256::MIN)
    );
}
