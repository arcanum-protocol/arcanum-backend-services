use super::*;
use ethers::prelude::*;
use std::fmt::{self, Display};

impl fmt::Display for Multipool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Multipool:\n")?;
        write!(f, "  contract_address: {}\n", self.contract_address)?;
        write!(f, "  assets:\n")?;
        for asset in &self.assets {
            write!(f, "    {}\n", asset)?;
        }
        if let Some(total_supply) = &self.total_supply {
            write!(f, "  total_supply: {}\n", total_supply)?;
        }
        if let Some(total_shares) = &self.total_shares {
            write!(f, "  total_shares: {}\n", total_shares)?;
        }
        Ok(())
    }
}

impl fmt::Display for MultipoolAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MultipoolAsset:\n")?;
        write!(f, "  address: {}\n", self.address)?;
        if let Some(price) = &self.price {
            write!(f, "  price: {}\n", price)?;
        }
        if let Some(quantity_slot) = &self.quantity_slot {
            write!(f, "  quantity_slot: {}\n", quantity_slot)?;
        }
        if let Some(share) = &self.share {
            write!(f, "  share: {}\n", share)?;
        }
        Ok(())
    }
}

impl fmt::Display for QuantityData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "QuantityData:\n")?;
        write!(f, "quantity : {}\n", self.quantity)?;
        write!(f, "cashback : {}\n", self.cashback)?;
        Ok(())
    }
}

impl<V> fmt::Display for MayBeExpired<V>
where
    V: Display + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MayBeExpired {}", self.clone().any_age())?;
        Ok(())
    }
}

//fn are_maybe_expired_equal<V: PartialEq>(exp1: &MayBeExpired<V>, exp2: &MayBeExpired<V>) -> bool {
//    if let (Some(value1), Some(value2)) = (exp1.not_older_than(0), exp2.not_older_than(0)) {
//        value1 == value2
//    } else {
//        false
//    }
//}

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

#[test]
fn check_base() {
    let contract_address: H160 = Address::random();
    let base_assets_adresses: Vec<Address> = (0..20).map(|_| Address::random()).collect();
    let updated_prices: Vec<(Address, Price)> = (0..5)
        .map(|i| (Address::random(), U256::from(i * 20)))
        .collect();
    //let existing_addresses: Vec<Address> = vec![];
    //let mut assets: Vec<MultipoolAsset> = vec![];
    let pow_x96 = |x: i64| U256::from(x) * (U256::pow(U256::from(2), U256::from(X96)));
    let mut mp_instance = Multipool::new(contract_address);
    mp_instance.update_prices(&updated_prices, false);
    let mut total_shares = U256::zero();
    let mut total_supply = U256::zero();
    let mut assets: Vec<MultipoolAsset> = Vec::new();
    for i in 0..10 {
        let share_number = pow_x96(10);
        let price_number = pow_x96(i);
        let quantity_data = QuantityData {
            quantity: pow_x96(10),
            cashback: pow_x96(10),
        };
        let asset = MultipoolAsset {
            address: base_assets_adresses[i as usize],
            price: Some(MayBeExpired::new(price_number)),
            quantity_slot: Some(MayBeExpired::new(quantity_data)),
            share: Some(MayBeExpired::new(share_number)),
        };
        total_supply += price_number;
        total_shares += share_number;
        assets.push(asset)
    }
    let other_instance = Multipool {
        contract_address,
        assets,
        total_supply: Some(MayBeExpired::new(total_supply)),
        total_shares: Some(MayBeExpired::new(total_shares)),
    };
    //check multipool are not equal
    assert_ne!(mp_instance, other_instance);
    //check multipool addresses are similar
    assert_eq!(
        mp_instance.contract_address(),
        other_instance.contract_address()
    );
    //check find asset
    assert_eq!(
        other_instance.asset(&base_assets_adresses[1]).unwrap(),
        other_instance
            .assets
            .iter()
            .find(|asset| asset.address.eq(&base_assets_adresses[1]))
            .cloned()
            .unwrap()
    );
    //NOTE ERRORS
    // check not existing addresses
    let random_address = Address::random();
    let result = other_instance.asset(&random_address);
    match result {
        Ok(_) => panic!("Expected AssetMissing, but got Ok"),
        Err(actual_error) => {
            assert_eq!(actual_error, MultipoolErrors::AssetMissing(random_address));
        }
    }
    //assert_eq!(
    //    other_instance.asset(&random_address),
    //    MultipoolErrors::AssetMissing(random_address)
    //)
}
