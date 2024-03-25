use crate::{expiry::TimeExtractor, MultipoolFees};

use super::{expiry::MayBeExpired, Multipool, MultipoolAsset, Price, QuantityData, Share};
use ethers::prelude::*;
use std::collections::HashMap;

impl<T: TimeExtractor> Multipool<T> {
    pub fn update_fees(&mut self, fees: Option<MultipoolFees>, update_expiry: bool) {
        if let Some(fees) = fees {
            self.fees = Some(MayBeExpired::new(fees))
        } else if update_expiry {
            if let Some(fees) = self.fees.as_mut() {
                fees.refresh();
            }
        }
    }

    /// Prices are updated if these assets present in pool. Otherwhise there is no effect
    pub fn update_prices(&mut self, prices: &[(Address, Price)], update_expiry: bool) {
        //TODO: replase with 0(max(len(prices), len(self.assets)))
        let prices_set: HashMap<Address, Price> = prices.iter().cloned().collect();
        for asset in self.assets.iter_mut() {
            if let Some(new_price) = prices_set.get(&asset.address).cloned() {
                asset.price = Some(MayBeExpired::new(new_price));
            } else if update_expiry {
                if let Some(price) = asset.price.as_mut() {
                    price.refresh();
                }
            }
        }
    }

    pub fn update_quantities(
        &mut self,
        quantities: &[(Address, QuantityData)],
        update_expiry: bool,
    ) {
        //TODO: replase with 0(max(len(quantities), len(self.assets)))
        let mut quantities_set: HashMap<Address, QuantityData> =
            quantities.iter().cloned().collect();

        if let Some(QuantityData {
            quantity: total_supply,
            cashback: _,
        }) = quantities_set.remove(&self.contract_address)
        {
            self.total_supply = Some(MayBeExpired::new(total_supply));
        } else if update_expiry {
            if let Some(total_supply) = self.total_supply.as_mut() {
                total_supply.refresh();
            }
        }

        self.assets = self
            .assets
            .clone()
            .into_iter()
            .filter_map(|mut asset| {
                if let Some(new_quantity_data) = quantities_set.remove(&asset.address) {
                    if new_quantity_data.is_empty() && asset.share.is_none() {
                        return None;
                    } else if new_quantity_data.is_empty() {
                        asset.quantity_slot = None;
                    } else {
                        asset.quantity_slot = Some(MayBeExpired::new(new_quantity_data));
                    }
                } else if update_expiry {
                    if let Some(quantity_slot) = asset.quantity_slot.as_mut() {
                        quantity_slot.refresh();
                    }
                }
                Some(asset)
            })
            .collect();
        self.assets.extend(
            quantities_set
                .into_iter()
                .map(|(address, slot)| MultipoolAsset {
                    address,
                    quantity_slot: Some(slot).filter(|s| !s.is_empty()).map(MayBeExpired::new),
                    price: Default::default(),
                    share: Default::default(),
                }),
        );
    }

    pub fn update_shares(&mut self, shares: &[(Address, Share)], update_expiry: bool) {
        //TODO: replase with 0(max(len(quantities), len(self.assets)))
        let mut shares_set: HashMap<Address, Share> = shares.iter().cloned().collect();
        let mut total_shares = self
            .total_shares
            .clone()
            .map(MayBeExpired::any_age)
            .unwrap_or(U256::zero());
        self.assets = self
            .assets
            .clone()
            .into_iter()
            .filter_map(|mut asset| {
                let old_share = asset
                    .share
                    .clone()
                    .map(MayBeExpired::any_age)
                    .unwrap_or(U256::zero());
                if let Some(new_share) = shares_set.remove(&asset.address) {
                    total_shares = total_shares.checked_sub(old_share).unwrap_or(U256::zero());
                    if new_share.is_zero() && asset.quantity_slot.is_none() {
                        return None;
                    } else if new_share.is_zero() {
                        asset.share = None;
                    } else {
                        asset.share = Some(MayBeExpired::new(new_share));
                        total_shares += new_share;
                    }
                } else if update_expiry {
                    if let Some(share) = asset.share.as_mut() {
                        share.refresh();
                    }
                }
                Some(asset)
            })
            .collect();
        self.total_shares = if total_shares.is_zero() {
            None
        } else {
            Some(MayBeExpired::new(total_shares))
        };
        self.assets.extend(
            shares_set
                .into_iter()
                .map(|(address, share)| MultipoolAsset {
                    address,
                    quantity_slot: Default::default(),
                    price: Default::default(),
                    share: Some(share).filter(|s| !s.is_zero()).map(MayBeExpired::new),
                }),
        );
    }
}
