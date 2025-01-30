use alloy::primitives::{aliases::U112, Address, U128, U256};
use multipool_types::Multipool::MultipoolEvents;

use super::{expiry::MayBeExpired, Multipool, MultipoolAsset};
use std::collections::HashMap;

impl Multipool {
    /// Prices are updated if these assets present in pool. Otherwhise there is no effect
    pub fn update_prices(&mut self, prices: &[(Address, U256)], timestamp: u64) {
        //TODO: replase with 0(max(len(prices), len(self.assets)))
        let prices_set: HashMap<Address, U256> = prices.iter().cloned().collect();
        for asset in self.assets.iter_mut() {
            if let Some(new_price) = prices_set.get(&asset.address).cloned() {
                asset.price = Some(MayBeExpired::with_time(new_price, timestamp));
            }
        }
    }

    pub fn apply_events(&mut self, events: &[MultipoolEvents]) {
        events.iter().for_each(|v| match v {
            MultipoolEvents::AssetChange(e) => {
                match self.assets.iter().position(|a| a.address.eq(&e.asset)) {
                    Some(idx) => {
                        self.assets[idx].quantity = U128::from(e.quantity);
                        self.assets[idx].collected_cashbacks = U112::from(e.collectedCashbacks);
                    }
                    None => {
                        let mut asset = MultipoolAsset::new(e.asset);
                        asset.quantity = U128::from(e.quantity);
                        asset.collected_cashbacks = U112::from(e.collectedCashbacks);
                        self.assets.push(asset);
                    }
                }
            }
            MultipoolEvents::FeesChange(e) => {
                self.deviation_increase_fee = e.newHalfDeviationFee;
                self.deviation_limit = e.newDeviationLimit;
                self.management_fee_receiver = e.newManagementFeeRecepientAddress;
                self.management_fee = e.newManagementFee;
                self.cashback_fee = e.newDepegBaseFee;
                self.base_fee = e.newBaseFee;
            }
            MultipoolEvents::PriceVerifierUpdated(e) => {
                self.oracle_address = e.newPriceVerifierAddress;
            }
            MultipoolEvents::TargetShareChange(e) => {
                self.total_target_shares = e.newTotalTargetShares;
                match self.assets.iter().position(|a| a.address.eq(&e.asset)) {
                    Some(idx) => {
                        self.assets[idx].share = e.newTargetShare;
                    }
                    None => {
                        let mut asset = MultipoolAsset::new(e.asset);
                        asset.share = e.newTargetShare;
                        self.assets.push(asset);
                    }
                }
            }
            MultipoolEvents::PriceFeedChange(e) => {
                match self
                    .assets
                    .iter()
                    .position(|a| a.address.eq(&e.targetAsset))
                {
                    Some(idx) => {
                        self.assets[idx].price_data = e.newFeed;
                    }
                    None => {
                        let mut asset = MultipoolAsset::new(e.targetAsset);
                        asset.price_data = e.newFeed;
                        self.assets.push(asset);
                    }
                }
            }
            _ => (),
        });
    }
}
