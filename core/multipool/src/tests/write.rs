use super::multipool_builder::{multipool_fixture, MultipoolMockBuilder};
use super::*;
use pretty_assertions::assert_eq;

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

//UPDATE PRICE
#[test]
fn check_update_prices() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = Multipool::new(contract_address);

    // for asset_initializing
    let quantity = U256::zero();
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];
    multipool.update_quantities(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];
    multipool.update_prices(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_price(ADDRESSES[0], U256::from(10) << 96)
        .set_price(ADDRESSES[1], U256::from(10) << 96)
        .set_price(ADDRESSES[2], U256::from(10) << 96)
        .set_price(ADDRESSES[3], U256::from(10) << 96)
        .set_price(ADDRESSES[4], U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_prices_with_not_existing_asset() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = Multipool::new(contract_address);

    // for asset_initializing
    let quantity = U256::zero();
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];
    multipool.update_quantities(&inserted_data, false);

    //insert base prices
    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];
    multipool.update_prices(&inserted_data, false);

    //insert fake price
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
        .set_price(ADDRESSES[0], U256::from(10) << 96)
        .set_price(ADDRESSES[1], U256::from(10) << 96)
        .set_price(ADDRESSES[2], U256::from(10) << 96)
        .set_price(ADDRESSES[3], U256::from(10) << 96)
        .set_price(ADDRESSES[4], U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_prices_with_custom_logic() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = Multipool::new(contract_address);

    // for asset_initializing
    let quantity = U256::zero();
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];
    multipool.update_quantities(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];
    multipool.update_prices(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::zero()),
        //skipped second asset
        (ADDRESSES[2], U256::from(15) << 96),
        (ADDRESSES[3], U256::from(20) << 96),
        (ADDRESSES[4], U256::from(25) << 96),
        (Address::random(), U256::from(10) << 96), // not exists
    ];
    multipool.update_prices(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_price(ADDRESSES[0], U256::zero())
        .set_price(ADDRESSES[1], U256::from(10) << 96)
        .set_price(ADDRESSES[2], U256::from(15) << 96)
        .set_price(ADDRESSES[3], U256::from(20) << 96)
        .set_price(ADDRESSES[4], U256::from(25) << 96)
        .build();

    assert_eq!(expected, multipool);
}

//UPDATE SHARES
#[test]
fn check_update_shares() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = Multipool::new(contract_address);

    // for asset_initializing, to set total_shares
    let quantity = U256::zero();
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];
    multipool.update_quantities(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];

    multipool.update_shares(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_share(ADDRESSES[0], U256::from(10) << 96)
        .set_share(ADDRESSES[1], U256::from(10) << 96)
        .set_share(ADDRESSES[2], U256::from(10) << 96)
        .set_share(ADDRESSES[3], U256::from(10) << 96)
        .set_share(ADDRESSES[4], U256::from(10) << 96)
        .set_total_shares(U256::from(50) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_shares_with_not_existing_asset() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = Multipool::new(contract_address);
    let random_address = Address::random();

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];
    multipool.update_shares(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::from(20) << 96),
        (ADDRESSES[1], U256::from(20) << 96),
        (ADDRESSES[2], U256::from(20) << 96),
        (ADDRESSES[3], U256::from(20) << 96),
        (ADDRESSES[4], U256::from(20) << 96),
        (random_address, U256::from(20) << 96), // not exists yet
    ];

    multipool.update_shares(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_share(ADDRESSES[0], U256::from(20) << 96)
        .set_share(ADDRESSES[1], U256::from(20) << 96)
        .set_share(ADDRESSES[2], U256::from(20) << 96)
        .set_share(ADDRESSES[3], U256::from(20) << 96)
        .set_share(ADDRESSES[4], U256::from(20) << 96)
        .set_share(random_address, U256::from(20) << 96) // expect that value was created
        .set_total_shares(U256::from(60) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_update_shares_with_custom_logic() {
    let random_address = Address::random();
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = Multipool::new(contract_address);

    // for asset_initializing, to set total_shares
    let quantity = U256::zero();
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];
    multipool.update_quantities(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];

    multipool.update_shares(&inserted_data, false);

    let inserted_data = [
        (ADDRESSES[0], U256::zero()),
        //skipped this asset ,
        (ADDRESSES[2], U256::from(15) << 96),
        (ADDRESSES[3], U256::from(20) << 96),
        (ADDRESSES[4], U256::from(25) << 96),
        (random_address, U256::from(20) << 96), // not exists
    ];

    multipool.update_shares(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES[1..5].to_vec())
        .set_share(ADDRESSES[1], U256::from(10) << 96)
        .set_share(ADDRESSES[2], U256::from(15) << 96)
        .set_share(ADDRESSES[3], U256::from(20) << 96)
        .set_share(ADDRESSES[4], U256::from(25) << 96)
        .set_share(random_address, U256::from(20) << 96)
        .set_total_shares(U256::from(70) << 96)
        .build();

    assert_eq!(expected, multipool);
}

//UPDATE QUANTITY
#[test]
fn check_add_quantities() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = Multipool::new(contract_address);

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];
    multipool.update_quantities(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_quantities_with_value(ADDRESSES.to_vec(), U256::from(10) << 96, None)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
fn check_add_quantities_with_total_supply() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = MultipoolMockBuilder::new(contract_address).build();

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
        (
            contract_address,
            QuantityData::new(U256::from(50) << 96, 0.into()),
        ),
    ];
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
    let mut multipool = Multipool::new(contract_address);

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];
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
    let mut multipool = Multipool::new(contract_address);

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
        (
            contract_address,
            QuantityData::new(U256::from(50) << 96, 0.into()),
        ),
    ];
    multipool.update_quantities(&inserted_data, false);

    //if we pass multiple contract_address data, last would be applied
    let total_supply = U256::from(40) << 96;
    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(U256::zero(), 0.into())),
        (ADDRESSES[4], QuantityData::new(U256::zero(), 0.into())),
        (
            contract_address,
            QuantityData::new(U256::from(70) << 96, 0.into()),
        ),
        (
            contract_address,
            QuantityData::new(U256::from(90) << 96, 0.into()),
        ),
        (contract_address, QuantityData::new(total_supply, 0.into())),
    ];
    multipool.update_quantities(&inserted_data, false);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_quantity(ADDRESSES[0], quantity)
        .set_quantity(ADDRESSES[1], quantity)
        .set_quantity(ADDRESSES[2], quantity)
        .set_quantity(ADDRESSES[3], quantity)
        .set_quantity(contract_address, total_supply)
        .build();

    assert_eq!(multipool, expected);
}

#[test]
fn check_update_quantities_delete_with_zero_shares() {
    let contract_address = H160::from_low_u64_be(0x10);
    let mut multipool = MultipoolMockBuilder::new(contract_address).build();

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[1], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[2], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[3], QuantityData::new(quantity, 0.into())),
        (ADDRESSES[4], QuantityData::new(quantity, 0.into())),
    ];

    multipool.update_quantities(&inserted_data, true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_quantity(ADDRESSES[0], quantity)
        .set_quantity(ADDRESSES[1], quantity)
        .set_quantity(ADDRESSES[2], quantity)
        .set_quantity(ADDRESSES[3], quantity)
        .set_quantity(ADDRESSES[4], quantity)
        .build();

    assert_eq!(expected, multipool);
}
