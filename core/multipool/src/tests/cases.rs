use super::multipool_builder::{multipool_fixture, read_method_fixture, MultipoolMockBuilder};
use super::*;
use ethers::prelude::*;
use lazy_static::lazy_static;
use pretty_assertions::{assert_eq, assert_ne};

lazy_static! {
    pub static ref ADDRESSES: [Address; 5] = [
        H160::from_low_u64_le(1),
        H160::from_low_u64_le(2),
        H160::from_low_u64_le(3),
        H160::from_low_u64_le(4),
        H160::from_low_u64_le(5),
    ];
}

//NOTE check write methods
#[test]
fn check_update_prices() {
    let contract_address = H160::from_low_u64_be(0x10);

    let values: Vec<U256> = (0..5).into_iter().map(|_| U256::from(50) << 96).collect();

    let inserted_data: Vec<(H160, U256)> = ADDRESSES
        .to_vec()
        .into_iter()
        .zip(values.into_iter())
        .collect();

    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices(inserted_data)
        .build();

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(50) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_shares() {
    let contract_address = H160::from_low_u64_be(0x10);

    let values: Vec<U256> = (0..5).into_iter().map(|_| U256::from(10) << 96).collect();
    let inserted_data: Vec<(H160, U256)> = ADDRESSES
        .to_vec()
        .into_iter()
        .zip(values.into_iter())
        .collect();

    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_shares(inserted_data)
        .build();

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_shares_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_quantities() {
    let contract_address = H160::from_low_u64_be(0x10);

    let values: Vec<QuantityData> = (0..5)
        .into_iter()
        .map(|_| QuantityData {
            quantity: U256::from(10) << 96,
            cashback: U256::zero(),
        })
        .collect();
    let inserted_data: Vec<(H160, QuantityData)> = ADDRESSES
        .to_vec()
        .into_iter()
        .zip(values.into_iter())
        .collect();

    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities(inserted_data.clone(), None)
        .build();

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::from(10) << 96, None)
        .build();

    assert_eq!(expected, multipool);

    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities(inserted_data, Some(contract_address))
        .build();

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10) << 96,
            Some(contract_address),
        )
        .build();

    assert_eq!(expected, multipool);
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
        U256::from_dec_str("4201854926370611818649680345474").unwrap()
    )
}

#[test]
fn check_deviation() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let deviation = multipool.deviation(&ADDRESSES[2].clone());
    assert_eq!(deviation.unwrap().any_age(), I256::from(-205507736))
}

#[test]
fn quantity_to_deviation_positive() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let quantity_to_deviation = multipool.quantity_to_deviation(&ADDRESSES[1], I256::from(10));
    assert_eq!(
        quantity_to_deviation.unwrap().any_age(),
        I256::from_dec_str("-633825300114114700748351602449").unwrap()
    )
}

#[test]
fn quantity_to_deviation_negative() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let quantity_to_deviation = multipool.quantity_to_deviation(&ADDRESSES[2], I256::from(-20));
    assert_eq!(
        quantity_to_deviation.unwrap().any_age(),
        I256::from_dec_str("-316912650057057350374175801252").unwrap()
    )
}

#[test]
fn check_current_share() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let current_share = multipool.current_share(&ADDRESSES[4].clone());
    assert_eq!(
        current_share.unwrap().any_age(),
        U256::from_dec_str("3595289123").unwrap()
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
    let expected_error = MultipoolErrors::QuantitySlotMissing(ADDRESSES[0].clone());
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
    let expected_error = MultipoolErrors::PriceMissing(ADDRESSES[0].clone());
    assert_eq!(Err(expected_error), multipool.cap());
}

#[test]
fn check_cap_overflow() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::MAX, //U256::from(100000000) << 96,
            None,
        )
        .set_quantity(contract_address, U256::MAX)
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::one())
        .build();
    let expected_error =
        MultipoolErrors::Overflow(errors::MultipoolOverflowErrors::QuotedQuantityOverflow);
    println!("in check");
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
