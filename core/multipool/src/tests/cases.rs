use std::time::Duration;

use super::multipool_builder::{
    compare_assets, multipool_fixture, read_method_fixture, MultipoolMockBuilder,
};
use super::*;
use ethers::prelude::*;
use lazy_static::lazy_static;
use pretty_assertions::assert_eq;

lazy_static! {
    pub static ref ADDRESSES: [Address; 5] = [
        H160::from_low_u64_le(1),
        H160::from_low_u64_le(2),
        H160::from_low_u64_le(3),
        H160::from_low_u64_le(4),
        H160::from_low_u64_le(5),
    ];
}

fn uint_template() -> [(H160, U256); 5] {
    [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ]
}

fn quantity_data_template() -> [(H160, QuantityData); 5] {
    let quantity = U256::from(10) << 96;
    [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ]
}

fn quantity_data_with_supply_template(contract_address: H160) -> [(H160, QuantityData); 6] {
    let quantity = U256::from(10) << 96;
    [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
        (
            contract_address,
            QuantityData::new(U256::from(50) << 96, 0.into()),
        ),
    ]
}

#[test]
fn check_multipool_initialization() {
    let contract_address = H160::from_low_u64_be(0x10);
    let expected = multipool_fixture(contract_address, ADDRESSES.to_vec(), U256::from(10) << 96);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .fill_updated_shares_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10) << 96,
            Some(contract_address),
        )
        .build();
    assert_eq!(expected, multipool)
}

//NOTE check write methods
#[test]
fn check_update_prices() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = uint_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .build();
    multipool.update_prices(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_prices_with_not_existing_asset() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = uint_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .build();
    multipool.update_prices(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
        (Address::random(), U256::from(10) << 96), // not exists
    ];

    multipool.update_prices(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_shares_with_not_existing_asset() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = uint_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .build();
    multipool.update_shares(&inserted_data, false);

    let random_address = Address::random();
    let inserted_data = [
        (ADDRESSES[0], U256::from(20) << 96),
        (ADDRESSES[1], U256::from(20) << 96),
        (ADDRESSES[2], U256::from(20) << 96),
        (ADDRESSES[3], U256::from(20) << 96),
        (ADDRESSES[4], U256::from(20) << 96),
        (random_address, U256::from(20) << 96), // not exists
    ];

    multipool.update_shares(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_shares(inserted_data.to_vec())
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_shares() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = uint_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .build();
    multipool.update_shares(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_shares_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_add_quantities() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = quantity_data_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_quantities(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::from(10) << 96, None)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_add_quantities_with_total_supply() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = quantity_data_with_supply_template(contract_address);

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_quantities(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10) << 96,
            Some(contract_address),
        )
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_delete_quantities() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = quantity_data_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_quantities(&inserted_data, false);

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(U256::zero(), 0.into())),
    ];

    multipool.update_quantities(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(ADDRESSES[0..4].to_vec(), U256::from(10) << 96, None)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_delete_quantities_with_total_supply() {
    let contract_address = H160::from_low_u64_be(0x10);

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();

    let inserted_data = quantity_data_with_supply_template(contract_address);
    multipool.update_quantities(&inserted_data, false);

    let quantity = U256::from(10) << 96;

    //if we pass multiple contract_address data, last would be applied
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(U256::zero(), 0.into())),
        (
            contract_address,
            QuantityData::new(U256::from(70) << 96, 0.into()),
        ),
        (
            contract_address,
            QuantityData::new(U256::from(90) << 96, 0.into()),
        ),
        (
            contract_address,
            QuantityData::new(U256::from(40) << 96, 0.into()),
        ),
    ];

    multipool.update_quantities(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
    ];

    let mut expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities(inserted_data.to_vec(), Some(contract_address))
        .build();

    let mut sorted_mp_assets = multipool.assets.clone();
    sorted_mp_assets.sort_by(compare_assets);
    multipool.assets = sorted_mp_assets;

    let mut sorted_expected_assets = expected.assets.clone();
    sorted_expected_assets.sort_by(compare_assets);
    expected.assets = sorted_expected_assets;

    assert_eq!(multipool, expected);
}

#[test]
fn check_update_expiry_quantities_with_supply() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = quantity_data_with_supply_template(contract_address);

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_quantities(&inserted_data, false);
    std::thread::sleep(Duration::from_secs(2));

    multipool.update_quantities(&inserted_data, true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10) << 96,
            Some(contract_address),
        )
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_expiry_quantities() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = quantity_data_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_quantities(&inserted_data, false);
    std::thread::sleep(Duration::from_secs(2));

    multipool.update_quantities(&inserted_data, true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::from(10) << 96, None)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_quantities_delete_with_zero_shares() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = quantity_data_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_quantities(&inserted_data, false);
    std::thread::sleep(Duration::from_secs(2));

    multipool.update_quantities(&inserted_data, true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::from(10) << 96, None)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_expiry_prices() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = uint_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_prices(&inserted_data, false);
    std::thread::sleep(Duration::from_secs(2));
    multipool.update_prices(&inserted_data, true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_expiry_shares() {
    let contract_address = H160::from_low_u64_be(0x10);

    let inserted_data = uint_template();

    let mut multipool = MultipoolMockBuilder::new(contract_address).build();
    multipool.update_shares(&inserted_data, false);
    std::thread::sleep(Duration::from_secs(2));
    multipool.update_shares(&[], true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_shares_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

//NOTE READ METHODS

#[test]
fn check_get_price() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let price = multipool.get_price(&ADDRESSES[2].clone());
    assert_eq!(
        price.unwrap().any_age(),
        U256::from_dec_str("4278320775770274230051373318144").unwrap()
    );
    let price = multipool.get_price(&contract_address);
    assert_eq!(
        price.unwrap().any_age(),
        U256::from_dec_str("4217424327562004493549204037774").unwrap()
    )
}

#[test]
fn check_deviation() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let deviation = multipool.deviation(&ADDRESSES[2].clone());
    assert_eq!(deviation.unwrap().any_age(), I256::from(-205541849))
}

#[test]
fn quantity_to_deviation_positive() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = multipool_fixture(contract_address, ADDRESSES.to_vec(), U256::from(10) << 96);
    let quantity_to_deviation = multipool.quantity_to_deviation(&ADDRESSES[1], I256::from(10));
    assert_eq!(
        quantity_to_deviation.unwrap().any_age(),
        I256::from_dec_str("9223372036854775808000").unwrap()
    )
}

#[test]
fn quantity_to_deviation_negative() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = multipool_fixture(contract_address, ADDRESSES.to_vec(), U256::from(10) << 96);
    let quantity_to_deviation = multipool.quantity_to_deviation(&ADDRESSES[1], I256::from(-10));
    assert_eq!(
        quantity_to_deviation.unwrap().any_age(),
        I256::from_dec_str("-9223372036854775808000").unwrap()
    )
}

#[test]
fn check_current_share() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let current_share = multipool.current_share(&ADDRESSES[4].clone());
    assert_eq!(
        current_share.unwrap().any_age(),
        U256::from_dec_str("3582016449").unwrap()
    )
}

#[test]
fn check_target_share() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let target_share = multipool.target_share(&ADDRESSES[4].clone());
    assert_eq!(target_share.unwrap().any_age(), U256::from(1717986918))
}

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
fn check_cap_quantity_slot_missing() {
    let contract_address = H160::from_low_u64_be(0x10);

    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .build();

    // check quantity slot missing
    let expected_error = MultipoolErrors::QuantitySlotMissing(ADDRESSES[0]);
    assert_eq!(Err(expected_error), multipool.cap());
}

#[test]
fn check_cap_price_missing() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10),
            Some(contract_address),
        )
        .build();
    let expected_error = MultipoolErrors::PriceMissing(ADDRESSES[0]);
    assert_eq!(Err(expected_error), multipool.cap());
}

#[test]
fn check_cap_overflow() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::MAX, None)
        .set_quantity(contract_address, U256::MAX)
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(2))
        .build();
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
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10),
            Some(contract_address),
        )
        .build();

    let expected_error = MultipoolErrors::ShareMissing(ADDRESSES[1]);
    assert_eq!(
        Err(expected_error),
        multipool.clone().target_share(&ADDRESSES[1].clone())
    );
}

// MULTIPOOL CURRENT SHARE ERROR
#[test]
fn check_current_share_zero_division() {
    let contract_address = H160::from_low_u64_be(0x10);

    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10))
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10),
            Some(contract_address),
        )
        .build();

    println!(
        "{:?} from current_share",
        multipool.current_share(&ADDRESSES[1])
    );
    let expected_error = MultipoolErrors::ZeroCap;
    assert_eq!(
        Err(expected_error),
        multipool.clone().current_share(&ADDRESSES[1].clone())
    );
}

// MULTIPOOL QUANTITY TO DEVIATION ERRORS
#[test]
fn check_quantity_to_deviation_overflow() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_shares_with_value(ADDRESSES.to_vec(), U256::from(10))
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10),
            Some(contract_address),
        )
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10))
        .build();

    let expected_error =
        MultipoolErrors::Overflow(errors::MultipoolOverflowErrors::TargetDeviationOverflow);

    assert_eq!(
        Err(expected_error),
        multipool.quantity_to_deviation(&ADDRESSES[1].clone(), I256::MIN)
    );
}
