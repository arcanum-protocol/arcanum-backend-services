use std::ops::Shr;

use ethers::{abi::Token, prelude::*};

use anyhow::{anyhow, bail, Result};

use crate::{
    contracts::{
        multipool::{AssetArgs, ForcePushArgs, MultipoolContract},
        SiloLens, ERC20,
    },
    trade::{AssetsChoise, MultipoolChoise, WrapperCall},
    uniswap::RETRIES,
};

pub const SILO_LENS: &str = "0xBDb843c7a7e48Dc543424474d7Aa63b61B5D9536";
pub const SILO_WRAPPER: &str = "0x5F127Aedf5A31E2F2685E49618D4f4809205fd62";

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

        let multipool_amount_in = U256::try_from(amount_of_in.abs())?;
        let multipool_amount_out = U256::try_from(amount_of_out.abs())?;

        let (unwrapped_amount_in, swap_asset_in, wrap_call) = if let Some((silo_pool, base_asset)) =
            self.trading_data.uniswap.silo_assets.get(&self.asset1)
        {
            let (total_supply, collected): (U256, U256) = self
                .trading_data
                .rpc
                .aquire(
                    |provider, _| async {
                        let total_supply = ERC20::new(self.asset1, provider.clone())
                            .total_supply()
                            .call()
                            .await?;
                        println!("{}, {}", silo_pool, base_asset);

                        let collected =
                            SiloLens::new(SILO_LENS.parse::<Address>().unwrap(), provider)
                                .total_deposits_with_interest(*silo_pool, *base_asset)
                                .call()
                                .await?;
                        anyhow::Ok((total_supply, collected))
                    },
                    RETRIES,
                )
                .await
                .map_err(|e| anyhow!(e))?;
            (
                multipool_amount_in * collected / total_supply,
                *base_asset,
                WrapperCall {
                    wrapper: SILO_WRAPPER.parse().unwrap(),
                    data: abi::encode(&[
                        Token::Address(*silo_pool),
                        Token::Address(*base_asset),
                        Token::Address(self.asset1),
                    ]),
                },
            )
        } else {
            (
                multipool_amount_in,
                self.asset1,
                WrapperCall {
                    wrapper: Address::zero(),
                    data: Vec::default(),
                },
            )
        };

        let (unwrapped_amount_out, swap_asset_out, unwrap_call) =
            if let Some((silo_pool, base_asset)) =
                self.trading_data.uniswap.silo_assets.get(&self.asset2)
            {
                let (total_supply, collected): (U256, U256) = self
                    .trading_data
                    .rpc
                    .aquire(
                        |provider, _| async {
                            let total_supply = ERC20::new(self.asset2, provider.clone())
                                .total_supply()
                                .call()
                                .await?;
                            println!("{}, {}", silo_pool, base_asset);

                            let collected =
                                SiloLens::new(SILO_LENS.parse::<Address>().unwrap(), provider)
                                    .total_deposits_with_interest(*silo_pool, *base_asset)
                                    .call()
                                    .await?;
                            anyhow::Ok((total_supply, collected))
                        },
                        RETRIES,
                    )
                    .await
                    .map_err(|e| anyhow!(e))?;
                (
                    multipool_amount_out * collected / total_supply,
                    *base_asset,
                    WrapperCall {
                        wrapper: SILO_WRAPPER.parse().unwrap(),
                        data: abi::encode(&[
                            Token::Address(*silo_pool),
                            Token::Address(*base_asset),
                            Token::Address(self.asset2),
                        ]),
                    },
                )
            } else {
                (
                    multipool_amount_out,
                    self.asset2,
                    WrapperCall {
                        wrapper: Address::zero(),
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
