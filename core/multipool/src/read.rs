use std::ops::{Shl, Shr};

use alloy::primitives::{Address, I256, U256, U512};

use crate::expiry::TimeExtractor;

use super::{
    errors::MultipoolErrors, errors::MultipoolOverflowErrors, expiry::MayBeExpired, Merge,
    Multipool, MultipoolAsset, Price, Share, X32, X96,
};

impl<T: TimeExtractor> Multipool<T> {
    pub fn asset(&self, asset_address: &Address) -> Result<MultipoolAsset<T>, MultipoolErrors> {
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
    pub fn cap(&self) -> Result<MayBeExpired<Price, T>, MultipoolErrors> {
        let merged_prices = self
            .assets
            .iter()
            .map(|asset| -> Result<_, MultipoolErrors> { asset.quoted_quantity() })
            .collect::<Result<Vec<_>, MultipoolErrors>>()?;
        Ok(merged_prices.iter().fold(
            MayBeExpired::new(Default::default()),
            |a, b| -> MayBeExpired<U256, T> {
                (a, b.clone())
                    .merge(|(a, b)| a.checked_add(b))
                    .transpose()
                    .expect("should never be error, overflow will be earlier")
            },
        ))
    }

    /// Returns optional price that may be expired, returns None if there is no such asset
    pub fn get_price(
        &self,
        asset_address: &Address,
    ) -> Result<MayBeExpired<Price, T>, MultipoolErrors> {
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
    ) -> Result<MayBeExpired<Share, T>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        (asset.quoted_quantity()?, self.cap()?)
            .merge(|(q, c)| q.shl(X32).checked_div(c))
            .transpose()
            .ok_or(MultipoolErrors::ZeroCap)
    }

    pub fn target_share(
        &self,
        asset_address: &Address,
    ) -> Result<MayBeExpired<Share, T>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        let share = asset
            .share
            .ok_or(MultipoolErrors::ShareMissing(*asset_address))?;
        let total_shares = self
            .total_shares
            .clone()
            .expect("total_shares should always exists")
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
    ) -> Result<MayBeExpired<I256, T>, MultipoolErrors> {
        (
            self.target_share(asset_address)?,
            self.current_share(asset_address)?,
        )
            .merge(|(t, c)| -> Result<I256, MultipoolErrors> {
                let current_share = I256::try_from(c).expect("should always convert, shr X32");
                let target_share = I256::try_from(t).expect("should always convert, shr X32");
                Ok(current_share
                    .checked_sub(target_share)
                    .expect("should never overflow, both values are x32 values below hundred"))
            })
            .transpose()
    }

    // TODO: rewrite with merging
    pub fn quantity_to_deviation(
        &self,
        asset_address: &Address,
        target_deviation: I256,
    ) -> Result<MayBeExpired<I256, T>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        let quantity = asset
            .quantity_slot
            //.unwrap_or(MayBeExpired::new(QuantityData {
            //    quantity: U256::default(),
            //    cashback: U256::default(),
            //}))
            .ok_or(MultipoolErrors::QuantitySlotMissing(*asset_address))?
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
            .ok_or(MultipoolErrors::TotalSharesMissing(*asset_address))?
            .any_age();
        let usd_cap = U512::from(self.cap()?.any_age());
        let share_bound = U256::try_from(target_deviation.checked_abs().ok_or(
            MultipoolErrors::Overflow(MultipoolOverflowErrors::TargetDeviationOverflow),
        )?)
        .map_err(|_| MultipoolErrors::Overflow(MultipoolOverflowErrors::TargetDeviationOverflow))?
        .checked_mul(total_shares)
        .ok_or(MultipoolErrors::Overflow(
            MultipoolOverflowErrors::TotalSharesOverflow(self.contract_address),
        ))
        .map_err(|_| {
            MultipoolErrors::Overflow(MultipoolOverflowErrors::TotalSharesOverflow(
                self.contract_address,
            ))
        })?
        .shr(32);

        let result_share = if target_deviation.ge(&I256::default()) {
            U512::from(
                share
                    .checked_add(share_bound)
                    .unwrap_or_default()
                    .min(total_shares),
            )
        } else {
            U512::from(share.checked_sub(share_bound).unwrap_or_default())
        };

        let amount: I256 = I256::from_raw(
            (result_share
                .checked_mul(usd_cap.shl(96))
                .expect("multiply shouldn't overflow"))
            .checked_div(U512::from(price))
            .expect("price division shouldn't overflow")
            .checked_div(U512::from(total_shares))
            .expect("total shares division shouldn't overflow")
            .to(),
        );
        let amount = amount - I256::from_raw(quantity);
        Ok(MayBeExpired::new(amount))
    }
}
