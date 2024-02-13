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

    // TODO: calculate quantity to balance
    fn quantity_to_deviation(
        &self,
        asset_address: &Address,
        target_deviation: I256,
    ) -> Option<MayBeExpired<I256>> {
        todo!();
    }

    pub fn find_best_quantities(&self, quote_amount: U256, interval: u64) -> Option<()> {
        todo!();
        // let mut data: Vec<(I256, U256, MultipoolAsset)> = self
        //     .assets
        //     .clone()
        //     .into_iter()
        //     .map(|asset| -> Option<_> {
        //         Some((
        //             self.quantity_to_balance(&asset.address)?
        //                 .not_older_than(interval)?,
        //             self.target_share(&asset.address)?
        //                 .not_older_than(interval)?,
        //             asset,
        //         ))
        //     })
        //     .collect::<Option<Vec<_>>>()?;
        // data.sort_by_key(|a| a.0);
        // //data.into_iter().filter(|(quantity_to_bal, _, _)| quantity_to_bal.is_negative()).take_while(||)
        // Some(())
    }
}
