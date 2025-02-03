use super::multipool_builder::MultipoolMockBuilder;
use crate::errors::MultipoolErrors::*;
use crate::errors::MultipoolOverflowErrors::*;
use crate::tests::ADDRESSES;
use alloy::primitives::Address;
use alloy::primitives::I256;
use alloy::primitives::U256;
use pretty_assertions::assert_eq;

#[test]
fn check_asset_missing() {
    let contract_address = Address::from_slice(&[0x10; 20]);
    let multipool = MultipoolMockBuilder::new(contract_address);
    let expected_error = AssetMissing(contract_address);
    assert_eq!(
        multipool.build().asset(&contract_address),
        Err(expected_error),
    );
}

#[test]
fn check_cap_quantity_slot_missing() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());

    let multipool = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .build();

    let expected_error = QuantitySlotMissing(ADDRESSES[0]);
    assert_eq!(multipool.cap(), Err(expected_error),);
}

#[test]
fn check_cap_price_missing() {
    let contract_address = Address::from_slice(10_u64.to_be_bytes().as_ref());
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
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
    let multipool = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::MAX, None)
        .set_quantity(contract_address, U256::MAX)
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(1) << 96)
        .build();
    let expected_error = Overflow(QuotedQuantityOverflow);
    assert_eq!(Err(expected_error), multipool.cap());
}

#[test]
fn check_target_share_share_missing() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
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

#[test]
fn check_current_share_zero_division() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());

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

#[test]
fn check_quantity_to_deviation_overflow() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
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
