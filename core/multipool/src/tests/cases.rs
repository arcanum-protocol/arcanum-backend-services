use super::multipool_builder::{read_method_fixture, MultipoolMockBuilder};
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

#[test]
fn check_base() {
    let contract_address = H160::from_low_u64_be(0x10);
    let expected = MultipoolMockBuilder::new(contract_address).multipool_fixture(
        contract_address,
        ADDRESSES.to_vec(),
        U256::from(10) << 96,
    );
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

//NOTE happy_path_read
#[test]
//build new instance of multipool with read methods
fn happy_path_read() {
    let contract_address = H160::from_low_u64_be(0x10);
    let expected = MultipoolMockBuilder::new(contract_address)
        .insert_assets(ADDRESSES.to_vec())
        .fill_updated_prices_with_value(ADDRESSES.to_vec(), U256::from(10))
        .fill_updated_shares_with_value(ADDRESSES.to_vec(), U256::from(10))
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10),
            Some(contract_address),
        )
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
    assert_eq!(expected, multipool);
    todo!("think about diffrent pathes and checks");
}

//NOTE

#[test]
fn check_deviation() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let deviation = multipool.deviation(&ADDRESSES[2].clone());
    assert_eq!(
        deviation.unwrap().any_age(),
        I256::from_dec_str("-205548970").unwrap()
    )
}

#[test]
fn quantity_to_deviation() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let quantity_to_deviation = multipool.quantity_to_deviation(&ADDRESSES[1], I256::from(10));
    assert_eq!(
        quantity_to_deviation.unwrap().any_age(),
        I256::from_dec_str("-633825300114114700748351602448").unwrap()
    )
}

#[test]
fn check_current_share() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let current_share = multipool.current_share(&ADDRESSES[4].clone());
    assert_eq!(
        current_share.unwrap().any_age(),
        U256::from_dec_str("3579245887").unwrap()
    )
}

#[test]
fn check_target_share() {
    let contract_address = H160::from_low_u64_be(0x10);
    let multipool = read_method_fixture(contract_address);
    let target_share = multipool.target_share(&ADDRESSES[4].clone());
    println!("{:?}", target_share);
    assert_eq!(
        target_share.unwrap().any_age(),
        U256::from_dec_str("1717986918").unwrap()
    )
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
        .fill_updated_quantities_with_value(
            ADDRESSES.to_vec(),
            U256::from(10),
            Some(contract_address),
        )
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
