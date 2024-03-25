use super::multipool_builder::{multipool_fixture, read_method_fixture};
use super::*;
use pretty_assertions::assert_eq;

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
