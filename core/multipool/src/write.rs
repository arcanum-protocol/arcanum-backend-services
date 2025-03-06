use alloy::{
    primitives::{aliases::U112, Address, U128, U256},
    rpc::types::Filter,
    sol_types::SolEvent,
};
use multipool_types::Multipool::MultipoolEvents;

use crate::expiry::EmptyTimeExtractor;

use super::{expiry::MayBeExpired, Multipool, MultipoolAsset};
use std::collections::HashMap;

impl Multipool {
    /// Prices are updated if these assets present in pool. Otherwhise there is no effect
    pub fn update_prices(&mut self, prices_set: HashMap<Address, U256>, timestamp: u64) {
        //TODO: replase with 0(max(len(prices), len(self.assets)))
        for asset in self.assets.iter_mut() {
            if let Some(new_price) = prices_set.get(&asset.address).cloned() {
                asset.price = Some(MayBeExpired::with_time(new_price, timestamp));
            }
        }
    }

    pub fn update_maybe_prices(
        &mut self,
        prices_set: &HashMap<Address, MayBeExpired<U256, EmptyTimeExtractor>>,
    ) {
        //TODO: replase with 0(max(len(prices), len(self.assets)))
        for asset in self.assets.iter_mut() {
            if let Some(new_price) = prices_set.get(&asset.address).cloned() {
                asset.price = Some(new_price);
            }
        }
    }

    pub fn filter() -> Filter {
        use multipool_types::Multipool::*;
        Filter::new().events([
            PoolCreated::SIGNATURE,
            TargetShareChange::SIGNATURE,
            AssetChange::SIGNATURE,
            FeesChange::SIGNATURE,
            PriceOracleChange::SIGNATURE,
            StrategyManagerChange::SIGNATURE,
            OwnershipTransferred::SIGNATURE,
            PriceFeedChange::SIGNATURE,
            Swap::SIGNATURE,
        ])
    }

    pub fn apply_events(&mut self, events: &[MultipoolEvents]) {
        events.iter().for_each(|v| match v {
            MultipoolEvents::PoolCreated(e) => {
                println!("pool created {:?}", e.initialSharePrice);
                self.initial_share_price = e.initialSharePrice;
            }
            MultipoolEvents::AssetChange(e) => {
                if self.contract_address == e.asset {
                    self.total_supply = U256::from(e.quantity);
                } else {
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
            }
            MultipoolEvents::FeesChange(e) => {
                self.deviation_increase_fee = e.newDeviationIncreaseFee;
                self.deviation_limit = e.newDeviationLimit;
                self.management_fee_receiver = e.newManagementFeeRecepient;
                self.management_fee = e.newManagementFee;
                self.cashback_fee = e.newFeeToCashbackRatio;
                self.base_fee = e.newBaseFee;
            }
            MultipoolEvents::PriceOracleChange(e) => {
                self.oracle_address = e.newOracle;
            }
            MultipoolEvents::OwnershipTransferred(e) => {
                self.owner = e.newOwner;
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
            MultipoolEvents::StrategyManagerChange(e) => {
                self.strategy_manager = e.newStrategyManager;
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
