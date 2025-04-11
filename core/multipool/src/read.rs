use std::ops::Shl;

use alloy::primitives::{Address, I256, U256, U512};

use multipool_types::expiry::{EmptyTimeExtractor, MayBeExpired};

use super::{
    errors::MultipoolErrors, errors::MultipoolOverflowErrors, Merge, Multipool, MultipoolAsset,
    X32, X96,
};

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

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn asset_list(&self) -> Vec<Address> {
        self.assets
            .clone()
            .into_iter()
            .map(|assets| assets.address)
            .collect()
    }

    /// Returns optional price that may be expired, returns None if there is no such asset
    pub fn cap(&self) -> Result<MayBeExpired<U256, EmptyTimeExtractor>, MultipoolErrors> {
        let merged_prices = self
            .assets
            .iter()
            .map(|asset| -> Result<_, MultipoolErrors> { asset.quoted_quantity() })
            .collect::<Result<Vec<_>, MultipoolErrors>>()?;
        Ok(merged_prices.iter().fold(
            MayBeExpired::with_time(Default::default(), u64::MAX),
            |a, b| -> MayBeExpired<U256, EmptyTimeExtractor> {
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
    ) -> Result<MayBeExpired<U256, EmptyTimeExtractor>, MultipoolErrors> {
        if self.contract_address.eq(asset_address) {
            Ok(self.cap()?.map(|c| {
                c.shl(X96)
                    .checked_div(self.total_supply)
                    .unwrap_or_default()
            }))
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
    ) -> Result<MayBeExpired<U256, EmptyTimeExtractor>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        (asset.quoted_quantity()?, self.cap()?)
            .merge(|(q, c)| q.shl(&X32).checked_div(c))
            .transpose()
            .ok_or(MultipoolErrors::ZeroCap)
    }

    pub fn target_share(&self, asset_address: &Address) -> Result<U256, MultipoolErrors> {
        U256::from(self.asset(asset_address)?.share)
            .shl(X32)
            .checked_div(U256::from(self.total_target_shares))
            .ok_or(MultipoolErrors::Overflow(
                MultipoolOverflowErrors::TotalSupplyOverflow,
            ))
    }

    pub fn deviation(
        &self,
        asset_address: &Address,
    ) -> Result<MayBeExpired<I256, EmptyTimeExtractor>, MultipoolErrors> {
        self.current_share(asset_address)?
            .map(|c| -> Result<I256, MultipoolErrors> {
                let t = self.target_share(asset_address)?;
                let current_share = I256::try_from(c).expect("should always convert, shr X32");
                let target_share = I256::try_from(t).expect("should always convert, shr X32");
                Ok(current_share
                    .checked_sub(target_share)
                    .expect("should never overflow, both values are x32 values below hundred"))
            })
            .transpose()
    }

    pub fn quantity_to_deviation(
        &self,
        asset_address: &Address,
        target_deviation: I256,
    ) -> Result<MayBeExpired<I256, EmptyTimeExtractor>, MultipoolErrors> {
        let asset = self.asset(asset_address)?;
        let quantity = asset.quantity;
        let price = asset
            .price
            .ok_or(MultipoolErrors::PriceMissing(*asset_address))?
            .any_age();
        let share = asset.share;
        let total_target_shares = self.total_target_shares;

        let usd_cap = U512::from(self.cap()?.any_age());

        let share_bound = U256::try_from(target_deviation.checked_abs().ok_or(
            MultipoolErrors::Overflow(MultipoolOverflowErrors::TargetDeviationOverflow),
        )?)
        .map_err(|_| MultipoolErrors::Overflow(MultipoolOverflowErrors::TargetDeviationOverflow))?
        .checked_mul(U256::from(total_target_shares))
        .ok_or(MultipoolErrors::Overflow(
            MultipoolOverflowErrors::TotalSharesOverflow(self.contract_address),
        ))
        .map_err(|_| {
            MultipoolErrors::Overflow(MultipoolOverflowErrors::TotalSharesOverflow(
                self.contract_address,
            ))
        })?
        .checked_shr(32)
        .ok_or(MultipoolErrors::Overflow(
            MultipoolOverflowErrors::TotalSharesOverflow(self.contract_address),
        ))?;

        let result_share = if target_deviation.ge(&I256::default()) {
            U512::from(
                U256::from(share)
                    .checked_add(share_bound)
                    .unwrap_or_default()
                    .min(U256::from(total_target_shares)),
            )
        } else {
            U512::from(
                U256::from(share)
                    .checked_sub(share_bound)
                    .unwrap_or_default(),
            )
        };

        let amount: I256 = I256::from_raw(
            (result_share
                .checked_mul(usd_cap.shl(96))
                .expect("multiply shouldn't overflow"))
            .checked_div(U512::from(price))
            .expect("price division shouldn't overflow")
            .checked_div(U512::from(total_target_shares))
            .expect("total shares division shouldn't overflow")
            .to(),
        );
        let amount = amount - I256::from_raw(U256::from(quantity));
        Ok(MayBeExpired::with_time(amount, 0))
    }
}
