use std::ops::Shl;

use super::{expiry::MayBeExpired, merge, Multipool, MultipoolAsset, Price, Share, X32, X96};
use ethers::prelude::*;

impl Multipool {
    pub fn asset(&self, asset_address: &Address) -> Option<MultipoolAsset> {
        self.assets
            .iter()
            .find(|asset| asset.address.eq(asset_address))
            .cloned()
    }

    pub fn contract_address(&self) -> Address {
        self.contract_address
    }

    pub fn asset_list(&self) -> Vec<Address> {
        self.assets
            .clone()
            .into_iter()
            .map(|assets| assets.address)
            .collect()
    }

    /// Returns optional price that may be expired, returns None if there is no such asset
    pub fn cap(&self) -> Option<MayBeExpired<Price>> {
        self.assets
            .iter()
            .filter_map(MultipoolAsset::quoted_quantity)
            .map(Option::from)
            .reduce(|a, b| merge(a, b, |a, b| a.merge(b, |a, b| a.checked_add(b)).transpose()))
            .flatten()
    }

    /// Returns optional price that may be expired, returns None if there is no such asset
    pub fn get_price(&self, asset_address: &Address) -> Option<MayBeExpired<Price>> {
        if self.contract_address.eq(asset_address) {
            merge(self.cap(), self.total_supply.as_ref(), |cap, ts| {
                cap.merge(ts.clone(), |c, t| c.shl(X96).checked_div(t))
                    .transpose()
            })
        } else {
            self.asset(asset_address).map(|asset| asset.price).flatten()
        }
    }

    pub fn current_share(&self, asset_address: &Address) -> Option<MayBeExpired<Share>> {
        let asset = self.asset(asset_address)?;
        merge(asset.quoted_quantity(), self.cap(), |q, c| {
            q.merge(c, |q, c| q.shl(X32).checked_div(c)).transpose()
        })
    }

    pub fn target_share(&self, asset_address: &Address) -> Option<MayBeExpired<Share>> {
        let asset = self.asset(asset_address)?;
        merge(asset.share, self.total_shares.clone(), |s, t| {
            s.merge(t, |s, t| s.shl(X32).checked_div(t)).transpose()
        })
    }

    pub fn deviation(&self, asset_address: &Address) -> Option<MayBeExpired<I256>> {
        merge(
            self.target_share(asset_address),
            self.current_share(asset_address),
            |t, c| {
                t.merge(c, |t, c| {
                    merge(I256::try_from(t).ok(), I256::try_from(c).ok(), |t, c| {
                        c.checked_sub(t)
                    })
                })
                .transpose()
            },
        )
    }

    // TODO: rewrite with merging
    pub fn quantity_to_deviation(
        &self,
        asset_address: &Address,
        target_deviation: I256,
        poison_time: u64,
    ) -> Option<MayBeExpired<I256>> {
        let asset = self.asset(asset_address)?;
        let quantity = asset
            .quantity_slot
            .unwrap()
            .not_older_than(poison_time)?
            .quantity;
        let price = asset.price.unwrap().not_older_than(poison_time)?;
        let share = asset.share.unwrap().not_older_than(poison_time)?;
        let total_shares = self
            .total_shares
            .clone()
            .unwrap()
            .not_older_than(poison_time)?;

        let usd_cap = self.cap().unwrap().not_older_than(poison_time)?;

        let share_bound =
            (U256::try_from(target_deviation.checked_abs()?).ok()? * total_shares) >> 32;
        let amount = if target_deviation.gt(&I256::from(0)) {
            I256::from_raw((share + share_bound).min(total_shares) * usd_cap / price / total_shares)
                - I256::from_raw(quantity)
        } else {
            I256::from_raw(
                (share.checked_sub(share_bound).unwrap_or(U256::zero())) * usd_cap
                    / price
                    / total_shares,
            ) - I256::from_raw(quantity)
        };
        Some(amount.into())
    }
}
