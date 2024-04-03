use std::ops::Shr;

use ethers::prelude::*;

use anyhow::{anyhow, bail, Result};

use crate::{
    contracts::multipool::{AssetArgs, ForcePushArgs, MultipoolContract},
    trade::{AssetsChoise, MultipoolChoise},
    uniswap::RETRIES,
};

impl<'a> AssetsChoise<'a> {
    pub async fn estimate_multipool(&self) -> Result<MultipoolChoise> {
        let price1 = self
            .trading_data
            .multipool
            .get_price(&self.asset1)
            .map_err(|v| anyhow!("{v:?}"))?
            .not_older_than(180)
            .ok_or(anyhow!("price too old"))?;
        let price2 = self
            .trading_data
            .multipool
            .get_price(&self.asset2)
            .map_err(|v| anyhow!("{v:?}"))?
            .not_older_than(180)
            .ok_or(anyhow!("price too old"))?;

        let amount1 = self
            .trading_data
            .multipool
            .quantity_to_deviation(&self.asset1, self.deviation_bound)
            .map_err(|v| anyhow!("{v:?}"))?
            .not_older_than(180)
            .ok_or(anyhow!("price too old"))?;

        let amount2 = self
            .trading_data
            .multipool
            .quantity_to_deviation(&self.asset2, self.deviation_bound)
            .map_err(|v| anyhow!("{v:?}"))?
            .not_older_than(180)
            .ok_or(anyhow!("price too old"))?;

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

        let quoted_amount1 = amount1
            .checked_mul(price1)
            .ok_or(anyhow!("overflow"))?
            .shr(96);
        let quoted_amount2 = amount2
            .checked_mul(price2)
            .ok_or(anyhow!("overflow"))?
            .shr(96);

        let quote_to_use = quoted_amount1.min(quoted_amount2);

        let amount_to_use = (quote_to_use << 96) / price1;

        let mut swap_args = vec![
            AssetArgs {
                asset_address: self.asset1,
                amount: I256::from_raw(amount_to_use),
            },
            AssetArgs {
                asset_address: self.asset2,
                amount: I256::from(-1000000i128),
            },
        ];

        println!("{} -> {}", self.asset1, self.asset2);
        println!("{} -> {}", amount1, amount2);

        swap_args.sort_by_key(|v| v.asset_address);

        let (fee, amounts): (I256, Vec<I256>) = self
            .trading_data
            .rpc
            .aquire(
                |provider, _| async {
                    let swap_args = swap_args.clone();
                    let force_push = self.trading_data.force_push.clone();
                    let force_push = ForcePushArgs {
                        contract_address: force_push.contract_address,
                        share_price: force_push.share_price,
                        timestamp: force_push.timestamp,
                        signatures: force_push.signatures,
                    };

                    MultipoolContract::new(self.trading_data.multipool.contract_address(), provider)
                        .check_swap(force_push, swap_args, true)
                        .call()
                        .await
                },
                RETRIES,
            )
            .await
            .map_err(|e| anyhow!(e))?;

        println!("out {:?}", amounts);

        let amount_of_in = amounts[0].max(amounts[1]);
        let amount_of_out = amounts[0].min(amounts[1]);

        Ok(MultipoolChoise {
            trading_data_with_assets: self,
            amount_in: U256::try_from(amount_of_in.abs())?,
            amount_out: U256::try_from(amount_of_out.abs())?,
            fee,
        })
    }
}
