use alloy::{
    primitives::{aliases::U112, Address, U128, U256},
    rpc::types::Filter,
    sol_types::SolEvent,
};
use multipool_types::Multipool::MultipoolEvents;

use multipool_types::expiry::{EmptyTimeExtractor, MayBeExpired};

use super::{Multipool, MultipoolAsset};

impl Multipool {
    pub fn update_prices(
        &mut self,
        prices: &Vec<(Address, MayBeExpired<U256, EmptyTimeExtractor>)>,
    ) {
        let mut assets = self.assets.iter_mut().peekable();
        let mut prices = prices.into_iter().peekable();

        while let (Some(asset), Some(price)) = (assets.peek_mut(), prices.peek()) {
            match asset.address.cmp(&price.0) {
                std::cmp::Ordering::Greater => {
                    prices.next();
                }
                std::cmp::Ordering::Less => {
                    assets.next();
                }
                std::cmp::Ordering::Equal => {
                    asset.price = Some(price.1.clone());
                    prices.next();
                    assets.next();
                }
            }
        }
    }

    pub fn filter() -> Filter {
        use multipool_types::Multipool::*;
        Filter::new().events([
            multipool_types::MultipoolFactory::MultipoolCreated::SIGNATURE,
            PoolCreated::SIGNATURE,
            TargetShareChange::SIGNATURE,
            AssetChange::SIGNATURE,
            FeesChange::SIGNATURE,
            PriceOracleChange::SIGNATURE,
            StrategyManagerChange::SIGNATURE,
            OwnershipTransferred::SIGNATURE,
            PriceFeedChange::SIGNATURE,
            Swap::SIGNATURE,
            ShareTransfer::SIGNATURE,
        ])
    }

    pub fn apply_events(&mut self, events: &[MultipoolEvents]) {
        events.iter().for_each(|v| match v {
            MultipoolEvents::PoolCreated(e) => {
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
                self.total_target_shares = e.newTotalTargetShares.to::<u16>();
                match self.assets.iter().position(|a| a.address.eq(&e.asset)) {
                    Some(idx) => {
                        self.assets[idx].share = e.newTargetShare.to::<u16>();
                    }
                    None => {
                        let mut asset = MultipoolAsset::new(e.asset);
                        asset.share = e.newTargetShare.to::<u16>();
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
                        let position = self
                            .assets
                            .iter()
                            .rev()
                            .take_while(|a| a.address.gt(&asset.address))
                            .count();
                        self.assets.insert(position, asset);
                    }
                }
            }
            _ => (),
        });
    }
}
