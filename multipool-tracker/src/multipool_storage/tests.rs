use super::*;
use ethers::prelude::*;
use rand::Rng;

#[test]
fn check_smth() {
    //let mp_instance = Multipool::new(Address::random());
    let mut rng = rand::thread_rng();
    let new_addresses: Vec<Address> = (0..5).map(|_| Address::random()).collect();
    let existing_addresses: Vec<Address> = vec![];
    let mut assets = vec![];
    let c = |x| U256::from(x);
    let p = |x| U256::from(x) * (U256::pow(c(2), c(X96)));
    let mut total_shares = U256::zero();
    let mut total_supply = U256::zero();
    for i in 0..10 {
        let share_number = p(10); //U256::pow(c(2), c(X96)) * c(10);
        let price_number = p(i); //U256::pow(c(2), c(X96)) * c(i);
        let quantity_data = QuantityData {
            quantity: U256::from(rng.gen::<u64>()),
            cashback: U256::from(rng.gen::<u64>()),
        };
        let asset = MultipoolAsset {
            address: Address::random(),
            price: Some(MayBeExpired::from(price_number)),
            quantity_slot: Some(MayBeExpired::from(quantity_data)),
            share: Some(MayBeExpired::from(share_number)),
        };
        total_supply += price_number;
        total_shares += share_number;
        assets.push(asset)
    }
    let mut mp_instance = Multipool {
        contract_address: Address::random(),
        assets: assets.clone(),
        total_supply: Some(MayBeExpired::from(total_supply)),
        total_shares: Some(MayBeExpired::from(total_shares)),
    };
    //let prices_vec_old = mp
    mp_instance.update_prices(
        &[(new_addresses[0], p(150)), (new_addresses[1], p(100))],
        true,
    );
    let equal_elements = assets
        .iter()
        .zip(&mp_instance.assets)
        .all(|(a, b)| Some(a.price) == Some(b.price));
    //assert_eq!(mp_instance.assets.len(), assets.len())
}
