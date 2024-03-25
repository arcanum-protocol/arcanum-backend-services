use std::ops::Shl;

use super::{
    errors::MultipoolErrors, errors::MultipoolOverflowErrors, expiry::MayBeExpired, Merge,
    Multipool, MultipoolAsset, Price, Share, X32, X96,
};
use ethers::prelude::*;

impl Multipool {
    pub fn asset(&self, asset_address: &Address) -> Result<MultipoolAsset, MultipoolErrors> {
        self.assets
            .iter()
            .find(|asset| asset.address.eq(asset_address))
            .cloned()
            .ok_or(MultipoolErrors::AssetMissing(*asset_address))
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
    pub fn cap(&self) -> Result<MayBeExpired<Price>, MultipoolErrors> {
        let merged_prices = self
            .assets
            .iter()
            .map(|asset| -> Result<_, MultipoolErrors> { asset.quoted_quantity() })
            .collect::<Result<Vec<_>, MultipoolErrors>>()?;
        merged_prices.iter().try_fold(
            MayBeExpired::new(Default::default()),
            |a, b| -> Result<MayBeExpired<U256>, MultipoolErrors> {
                (a, b.clone())
                    .merge(|(a, b)| a.checked_add(b))
                    .transpose()
                    .ok_or(MultipoolErrors::Overflow(
                        MultipoolOverflowErrors::PriceCapOverflow,
                    ))
            },
        )
    }

    /// Returns optional price that may be expired, returns None if there is no such asset
    pub fn get_price(
        &self,
        asset_address: &Address,
    ) -> Result<MayBeExpired<Price>, MultipoolErrors> {
        if self.contract_address.eq(asset_address) {
            let total_supply = self
                .total_supply
                .as_ref()
                .ok_or(MultipoolErrors::TotalSupplyMissing(*asset_address))?;
            (self.cap()?, total_supply.clone())
                .merge(|(c, t)| c.shl(X96).checked_div(t))
                .transpose()
                .ok_or(MultipoolErrors::Overflow(
                    MultipoolOverflowErrors::TotalSupplyOverflow,
                ))
        } else {
            self.asset(asset_address).and_then(|asset| {
                asset
                    .price
                    .ok_or(MultipoolErrors::PriceMissing(*asset_address))
            })
        }
    }

    pub fn current_share(
        &self,
        asset_address: &Address,
    ) -> Result<MayBeExpired<Share>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        (asset.quoted_quantity()?, self.cap()?)
            .merge(|(q, c)| q.shl(X32).checked_div(c))
            .transpose()
            .ok_or(MultipoolErrors::Overflow(
                MultipoolOverflowErrors::TotalSupplyOverflow,
            ))
    }

    pub fn target_share(
        &self,
        asset_address: &Address,
    ) -> Result<MayBeExpired<Share>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        let share = asset
            .share
            .ok_or(MultipoolErrors::ShareMissing(*asset_address))?;
        let total_shares = self
            .total_shares
            .clone()
            .ok_or(MultipoolErrors::TotalSharesMissing(*asset_address))?
            .clone();
        (share, total_shares)
            .merge(|(s, t)| s.shl(X32).checked_div(t))
            .transpose()
            .ok_or(MultipoolErrors::Overflow(
                MultipoolOverflowErrors::TotalSupplyOverflow,
            ))
    }

    pub fn deviation(
        &self,
        asset_address: &Address,
    ) -> Result<MayBeExpired<I256>, MultipoolErrors> {
        (
            self.target_share(asset_address)?,
            self.current_share(asset_address)?,
        )
            .merge(|(t, c)| -> Result<I256, MultipoolErrors> {
                let current_share = I256::try_from(c).map_err(|_| {
                    MultipoolErrors::Overflow(MultipoolOverflowErrors::CurrentShareTooBig)
                })?;
                let target_share = I256::try_from(t).map_err(|_| {
                    MultipoolErrors::Overflow(MultipoolOverflowErrors::CurrentShareTooBig)
                })?;
                current_share
                    .checked_sub(target_share)
                    .ok_or(MultipoolErrors::Overflow(
                        MultipoolOverflowErrors::TargetShareTooBig,
                    ))
            })
            .transpose()
    }

    // TODO: rewrite with merging
    pub fn quantity_to_deviation(
        &self,
        asset_address: &Address,
        target_deviation: I256,
    ) -> Result<MayBeExpired<I256>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        let quantity = asset
            .quantity_slot
            .ok_or(MultipoolErrors::PriceMissing(*asset_address))?
            .any_age()
            .quantity;
        let price = asset
            .price
            .ok_or(MultipoolErrors::PriceMissing(*asset_address))?
            .any_age();
        let share = asset
            .share
            .ok_or(MultipoolErrors::ShareMissing(*asset_address))?
            .any_age();
        let total_shares = self
            .total_shares
            .clone()
            .ok_or(MultipoolErrors::ShareMissing(*asset_address))?
            .any_age();
        let usd_cap = self.cap()?.any_age();
        let share_bound = (U256::try_from(target_deviation.checked_abs().ok_or(
            MultipoolErrors::Overflow(MultipoolOverflowErrors::TargetShareTooBig),
        )?)
        .map_err(|_| MultipoolErrors::Overflow(MultipoolOverflowErrors::CurrentShareTooBig))?
            * total_shares)
            >> 32;
        let result_share = if target_deviation.ge(&I256::from(0)) {
            (share + share_bound).min(total_shares)
        } else {
            share.checked_sub(share_bound).unwrap_or(U256::zero())
        };
        let amount = I256::from_raw(result_share * (usd_cap << 96) / price / total_shares)
            - I256::from_raw(quantity);
        Ok(MayBeExpired::new(amount))
    }
}
