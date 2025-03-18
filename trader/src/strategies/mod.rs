use alloy::{
    dyn_abi::DynSolValue,
    primitives::{Address, I256, U256},
    providers::Provider,
};
use anyhow::{anyhow, bail, Result};
use multipool::expiry::StdTimeExtractor;
use std::ops::Shr;

use crate::{
    contracts::{SiloLens, ERC20, SILO_LENS, SILO_WRAPPER},
    trade::{AssetsChoise, MultipoolChoise, WrapperCall},
};

impl<P: Provider + Clone> AssetsChoise<P> {
    pub async fn estimate_multipool(self) -> Result<MultipoolChoise<P>> {
        let price1 = self
            .trading_data
            .multipool
            .get_price(&self.asset1)
            .map_err(|v| anyhow!("{v:?}"))?
            .any_age();
        let price2 = self
            .trading_data
            .multipool
            .get_price(&self.asset2)
            .map_err(|v| anyhow!("{v:?}"))?
            .any_age();

        let amount1 = self
            .trading_data
            .multipool
            .quantity_to_deviation(&self.asset1, self.deviation_bound)
            .map_err(|v| anyhow!("{v:?}"))?
            .not_older_than::<StdTimeExtractor>(180)
            .ok_or(anyhow!("Price is too old"))?;

        let amount2 = self
            .trading_data
            .multipool
            .quantity_to_deviation(&self.asset2, self.deviation_bound)
            .map_err(|v| anyhow!("{v:?}"))?
            .not_older_than::<StdTimeExtractor>(180)
            .ok_or(anyhow!("Price is too old"))?;

        println!("{} -> {}", self.asset1, self.asset2);
        println!("{} -> {}", amount1, amount2);

        if (amount1.is_positive() && amount2.is_positive())
            || (amount1.is_negative() && amount2.is_negative())
        {
            bail!(anyhow!("same signs"));
        }

        if amount1.is_negative() {
            bail!(anyhow!("amount1 is neg"));
        }

        let amount1 = U256::try_from(amount1.abs())?;
        let amount2 = U256::try_from(amount2.abs())?;

        let quoted_amount1: U256 = amount1
            .checked_mul(price1)
            .ok_or(anyhow!("overflow"))?
            .shr(96);
        let quoted_amount2: U256 = amount2
            .checked_mul(price2)
            .ok_or(anyhow!("overflow"))?
            .shr(96);

        let quote_to_use = quoted_amount1.min(quoted_amount2);

        let amount_to_use = (quote_to_use << 96) / price1;

        // TODO: calculate fee
        let fee: I256 = I256::unchecked_from(100000000_u128);

        let amount_of_in = I256::from_raw(amount_to_use);
        let amount_of_out = I256::unchecked_from(-1000000);

        let multipool_amount_in = U256::try_from(amount_of_in.abs())?;
        let multipool_amount_out = U256::try_from(amount_of_out.abs())?;

        let (unwrapped_amount_in, swap_asset_in, wrap_call) = if let Some((silo_pool, base_asset)) =
            self.trading_data.silo_assets.get(&self.asset1)
        {
            let total_supply = ERC20::new(self.asset1, self.trading_data.rpc.clone())
                .totalSupply()
                .call()
                .await?;
            let collected = SiloLens::new(SILO_LENS, self.trading_data.rpc.clone())
                .totalDepositsWithInterest(*silo_pool, *base_asset)
                .call()
                .await?;

            (
                multipool_amount_in * collected._totalDeposits / total_supply.value,
                *base_asset,
                WrapperCall {
                    wrapper: SILO_WRAPPER,
                    data: DynSolValue::Tuple(vec![
                        DynSolValue::Address(*silo_pool),
                        DynSolValue::Address(*base_asset),
                        DynSolValue::Address(self.asset1),
                    ])
                    .abi_encode(),
                },
            )
        } else {
            (
                multipool_amount_in,
                self.asset1,
                WrapperCall {
                    wrapper: Address::ZERO,
                    data: Vec::default(),
                },
            )
        };

        let (unwrapped_amount_out, swap_asset_out, unwrap_call) = if let Some((
            silo_pool,
            base_asset,
        )) =
            self.trading_data.silo_assets.get(&self.asset2)
        {
            let total_supply = ERC20::new(self.asset2, self.trading_data.rpc.clone())
                .totalSupply()
                .call()
                .await?;

            let collected = SiloLens::new(SILO_LENS, self.trading_data.rpc.clone())
                .totalDepositsWithInterest(*silo_pool, *base_asset)
                .call()
                .await?;

            (
                multipool_amount_out * collected._totalDeposits / total_supply.value,
                *base_asset,
                WrapperCall {
                    wrapper: SILO_WRAPPER,
                    data: DynSolValue::Tuple(vec![
                        DynSolValue::Address(*silo_pool),
                        DynSolValue::Address(*base_asset),
                        DynSolValue::Address(self.asset2),
                    ])
                    .abi_encode(),
                },
            )
        } else {
            (
                multipool_amount_out,
                self.asset2,
                WrapperCall {
                    wrapper: Address::ZERO,
                    data: Vec::default(),
                },
            )
        };

        Ok(MultipoolChoise {
            trading_data_with_assets: self,

            swap_asset_in,
            swap_asset_out,

            multipool_amount_in,
            multipool_amount_out,

            unwrapped_amount_in,
            unwrapped_amount_out,

            wrap_call,
            unwrap_call,

            fee,
        })
    }
}
