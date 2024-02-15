use std::collections::HashMap;

use super::{expiry::MayBeExpired, Multipool, MultipoolAsset, Price, QuantityData, Share};
use ethers::prelude::*;

impl Multipool {
    /// Prices are updated if these assets present in pool. Otherwhise there is no effect
    pub fn update_prices(&mut self, prices: &[(Address, Price)], update_expiry: bool) {
        //TODO: replase with 0(max(len(prices), len(self.assets)))
        let prices_set: HashMap<Address, Price> = prices.into_iter().cloned().collect();
        for asset in self.assets.iter_mut() {
            if let Some(new_price) = prices_set.get(&asset.address).cloned() {
                asset.price = Some(new_price.into());
            } else if update_expiry {
                asset.price.as_mut().map(|v| v.refresh());
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
            quantities.into_iter().cloned().collect();

        if let Some(QuantityData {
            quantity: total_supply,
            cashback: _,
        }) = quantities_set.remove(&self.contract_address)
        {
            self.total_supply = Some(total_supply.into());
        } else if update_expiry {
            self.total_supply.as_mut().map(|v| v.refresh());
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
                        asset.quantity_slot = Some(new_quantity_data.into());
                    }
                } else if update_expiry {
                    asset.quantity_slot.as_mut().map(|v| v.refresh());
                }
                Some(asset)
            })
            .collect();
        self.assets.extend(
            quantities_set
                .into_iter()
                .map(|(address, slot)| MultipoolAsset {
                    address,
                    quantity_slot: Some(slot).filter(|s| !s.is_empty()).map(Into::into),
                    price: Default::default(),
                    share: Default::default(),
                }),
        );
    }

    pub fn update_shares(&mut self, shares: &[(Address, Share)], update_expiry: bool) {
        //TODO: replase with 0(max(len(quantities), len(self.assets)))
        let mut shares_set: HashMap<Address, Share> = shares.into_iter().cloned().collect();
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
                    total_shares -= old_share;
                    if new_share.is_zero() && asset.quantity_slot.is_none() {
                        return None;
                    } else if new_share.is_zero() {
                        asset.share = None;
                    } else {
                        asset.share = Some(new_share.into());
                        total_shares += new_share;
                    }
                } else if update_expiry {
                    asset.share.as_mut().map(|v| v.refresh());
                }
                Some(asset)
            })
            .collect();
        self.total_shares = if total_shares.is_zero() {
            None
        } else {
            Some(total_shares.into())
        };
        self.assets.extend(
            shares_set
                .into_iter()
                .map(|(address, share)| MultipoolAsset {
                    address,
                    quantity_slot: Default::default(),
                    price: Default::default(),
                    share: Some(share).filter(|s| !s.is_zero()).map(Into::into),
                }),
        );
    }
}
