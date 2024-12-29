use super::multipool_builder::MultipoolMockBuilder;
use super::*;
use pretty_assertions::assert_eq;

use super::multipool_builder::POISON_TIME;
use std::time::Duration;

#[test]
#[cfg(feature = "expiry")]
fn check_update_expiry_quantities() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
    let mut multipool = Multipool::new(contract_address);

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[1], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[2], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[3], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[4], QuantityData::new(quantity, U256::default())),
    ];

    multipool.update_quantities(&inserted_data, false);
    let expected = MultipoolMockBuilder::new(contract_address)
        .set_quantity(ADDRESSES[0], U256::from(10) << 96)
        .set_quantity(ADDRESSES[1], U256::from(10) << 96)
        .set_quantity(ADDRESSES[2], U256::from(10) << 96)
        .set_quantity(ADDRESSES[3], U256::from(10) << 96)
        .set_quantity(ADDRESSES[4], U256::from(10) << 96)
        .build();

    //sleep to convert MayBeExpired values to None
    std::thread::sleep(Duration::from_secs(POISON_TIME));

    assert_eq!(expected, multipool);
}

// multipool hasn't value, but expected has it, because POISON_TIME has passed
#[test]
#[cfg(feature = "expiry")]
fn check_update_expiry_quantities_with_1_sec_delay() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
    let mut multipool = MultipoolMockBuilder::new(contract_address).build();

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[1], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[2], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[3], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[4], QuantityData::new(quantity, U256::default())),
    ];
    multipool.update_quantities(&inserted_data, false);

    //sleep to convert MayBeExpired values in multipool to None
    std::thread::sleep(Duration::from_secs(1));

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_quantity(ADDRESSES[0], U256::from(10) << 96)
        .set_quantity(ADDRESSES[1], U256::from(10) << 96)
        .set_quantity(ADDRESSES[2], U256::from(10) << 96)
        .set_quantity(ADDRESSES[3], U256::from(10) << 96)
        .set_quantity(ADDRESSES[4], U256::from(10) << 96)
        .build();

    //sleep 1 sec less, because values in expected multipool have to be
    std::thread::sleep(Duration::from_secs(POISON_TIME - 1));

    assert_ne!(expected, multipool);
}

#[test]
#[cfg(feature = "expiry")]
fn check_update_expiry_quantities_with_supply() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
    let mut multipool = MultipoolMockBuilder::new(contract_address).build();

    let quantity = U256::from(10) << 96;
    let inserted_data = [
        (ADDRESSES[0], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[1], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[2], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[3], QuantityData::new(quantity, U256::default())),
        (ADDRESSES[4], QuantityData::new(quantity, U256::default())),
        (
            contract_address,
            QuantityData::new(U256::from(50) << 96, U256::default()),
        ),
    ];
    multipool.update_quantities(&inserted_data, false);

    std::thread::sleep(Duration::from_secs(POISON_TIME));

    //update after thread sleep
    //try to comment line below to see difference
    multipool.update_quantities(&inserted_data, true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_quantity(ADDRESSES[0], U256::from(10) << 96)
        .set_quantity(ADDRESSES[1], U256::from(10) << 96)
        .set_quantity(ADDRESSES[2], U256::from(10) << 96)
        .set_quantity(ADDRESSES[3], U256::from(10) << 96)
        .set_quantity(ADDRESSES[4], U256::from(10) << 96)
        .set_quantity(contract_address, U256::from(50) << 96)
        .build();

    assert_eq!(expected, multipool);
}

#[test]
#[cfg(feature = "expiry")]
fn check_update_expiry_prices() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
    let mut multipool = Multipool::new(contract_address);

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];

    multipool.update_prices(&inserted_data, false);

    //waiting 1 sec, to check that MayBeExpired values will be updated
    std::thread::sleep(Duration::from_secs(POISON_TIME - 1));

    // update values, comment this values
    multipool.update_prices(&inserted_data, true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10) << 96)
        .build();

    std::thread::sleep(Duration::from_secs(POISON_TIME - 1));

    assert_eq!(expected, multipool);
}

#[test]
#[cfg(feature = "expiry")]
fn check_update_expiry_shares() {
    let contract_address = Address::from_slice(0x10_u64.to_be_bytes().as_ref());
    let mut multipool = Multipool::new(contract_address);

    let inserted_data = [
        (ADDRESSES[0], U256::from(10) << 96),
        (ADDRESSES[1], U256::from(10) << 96),
        (ADDRESSES[2], U256::from(10) << 96),
        (ADDRESSES[3], U256::from(10) << 96),
        (ADDRESSES[4], U256::from(10) << 96),
    ];
    multipool.update_shares(&inserted_data, false);

    std::thread::sleep(Duration::from_secs(POISON_TIME));
    multipool.update_shares(&[], true);

    let expected = MultipoolMockBuilder::new(contract_address)
        .set_share(ADDRESSES[0], U256::from(10) << 96)
        .set_share(ADDRESSES[1], U256::from(10) << 96)
        .set_share(ADDRESSES[2], U256::from(10) << 96)
        .set_share(ADDRESSES[3], U256::from(10) << 96)
        .set_share(ADDRESSES[4], U256::from(10) << 96)
        .build();

    assert_eq!(expected, multipool);
}
