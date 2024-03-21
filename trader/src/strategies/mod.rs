use ethers::prelude::*;

use anyhow::{anyhow, Result};

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
            .quantity_to_deviation(&self.asset1, self.deviation_bound)
            .map_err(|v| anyhow!("{v:?}"))?
            .not_older_than(180)
            .ok_or(anyhow!("price too old"))?;

        let quoted_amount1: I256 = (amount1 * I256::from_raw(price1)) >> 96;
        let quoted_amount2: I256 = (amount2 * I256::from_raw(price2)) >> 96;

        let quote_to_use = quoted_amount1.abs().min(quoted_amount2.abs());

        let amount_to_use = (quote_to_use.abs() << 96) / I256::from_raw(price1);

        let mut swap_args = vec![
            AssetArgs {
                asset_address: self.asset1,
                amount: amount_to_use,
            },
            AssetArgs {
                asset_address: self.asset2,
                amount: I256::from(-10000000i128),
            },
        ];

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

        let amount_of_in = U256::try_from(amounts[1].max(amounts[0]).abs()).unwrap();
        let amount_of_out = U256::try_from(amounts[1].min(amounts[0]).abs()).unwrap();

        Ok(MultipoolChoise {
            trading_data_with_assets: self,
            amount_in: amount_of_in,
            amount_out: amount_of_out,
            fee,
        })
    }
}
