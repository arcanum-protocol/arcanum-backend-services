use super::multipool_builder::MultipoolMockBuilder;
use super::*;
use crate::errors::MultipoolErrors::*;
use crate::errors::MultipoolOverflowErrors::*;
use pretty_assertions::assert_eq;

//MULTIPOOL ASSET ERRORS
#[test]
fn check_asset_missing() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address);
    let expected_error = AssetMissing(contract_address);
    assert_eq!(
        multipool.build().asset(&contract_address),
        Err(expected_error),
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
    let expected_error = QuantitySlotMissing(ADDRESSES[0]);
    assert_eq!(multipool.cap(), Err(expected_error),);
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
    let expected_error = PriceMissing(ADDRESSES[0]);
    assert_eq!(Err(expected_error), multipool.cap());
}

#[test]
fn check_cap_overflow() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::MAX, None)
        .set_quantity(contract_address, U256::MAX)
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(1) << 96)
        .build();
    let expected_error = Overflow(QuotedQuantityOverflow);
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

    let expected_error = ShareMissing(ADDRESSES[1]);
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
    let expected_error = ZeroCap;

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

    let expected_error = Overflow(TargetDeviationOverflow);

    assert_eq!(
        Err(expected_error),
        multipool.quantity_to_deviation(&ADDRESSES[1].clone(), I256::MIN)
    );
}
