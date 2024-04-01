use anyhow::{anyhow, Result};

use ethers::prelude::*;

use crate::contracts::Quoter;

use crate::trade::{MultipoolChoise, PoolInfo, SwapOutcome, UniswapChoise};

pub const RETRIES: Option<usize> = Some(1);

pub enum AmountWithDirection {
    ExactInput(U256),
    ExactOutput(U256),
}

impl<'a> MultipoolChoise<'a> {
    pub async fn estimate_uniswap(&self) -> Result<UniswapChoise> {
        let uniswap = &self.trading_data_with_assets.trading_data.uniswap;
        let asset1 = self.trading_data_with_assets.asset1;
        let asset2 = self.trading_data_with_assets.asset2;
        let rpc = &self.trading_data_with_assets.trading_data.rpc;

        let input_uniswap_pool = uniswap.get_pool_fee(&asset1).map_err(|e| anyhow!(e))?;
        let output_uniswap_pool = uniswap.get_pool_fee(&asset2).map_err(|e| anyhow!(e))?;

        let result: Vec<U256> = rpc
            .aquire(
                move |provider, multicall_address| async move {
                    let input_uniswap_pool = uniswap
                        .get_pool_fee(&asset1)
                        .map_err(|e| anyhow!(e))
                        .unwrap();
                    let output_uniswap_pool = uniswap
                        .get_pool_fee(&asset2)
                        .map_err(|e| anyhow!(e))
                        .unwrap();

                    let quoter = Quoter::new(
                        "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
                            .parse::<Address>()
                            .unwrap(),
                        provider.clone(),
                    );
                    Multicall::new(provider, multicall_address)
                        .await
                        .unwrap()
                        .add_calls(
                            true,
                            input_uniswap_pool
                                .pools
                                .iter()
                                .map(|PoolInfo { fee, address: _ }| {
                                    quoter.quote_exact_output_single(
                                        self.trading_data_with_assets.trading_data.weth,
                                        asset1,
                                        *fee,
                                        self.amount_in,
                                        U256::zero(),
                                    )
                                }),
                        )
                        .add_calls(
                            true,
                            output_uniswap_pool
                                .pools
                                .iter()
                                .map(|PoolInfo { fee, address: _ }| {
                                    quoter.quote_exact_input_single(
                                        asset2,
                                        self.trading_data_with_assets.trading_data.weth,
                                        *fee,
                                        self.amount_out,
                                        U256::zero(),
                                    )
                                }),
                        )
                        .call_array()
                        .await
                },
                RETRIES,
            )
            .await?;

        let input_len = input_uniswap_pool.pools.len();
        let (best_input_pool, input_value) = input_uniswap_pool
            .pools
            .iter()
            .zip(result.iter().take(input_len))
            .min_by(|x, y| x.1.cmp(y.1))
            .ok_or(anyhow!("Pools not found"))?;

        let (best_output_pool, output_value) = output_uniswap_pool
            .pools
            .iter()
            .zip(result.iter().skip(input_len))
            .max_by(|x, y| x.1.cmp(y.1))
            .ok_or(anyhow!("Pools not found"))?;

        Ok(UniswapChoise {
            trading_data: self,
            input: SwapOutcome {
                estimated: *input_value,
                best_pool: best_input_pool.address,
                zero_for_one: input_uniswap_pool.base_is_token0,
                best_fee: best_input_pool.fee,
            },
            output: SwapOutcome {
                estimated: *output_value,
                best_pool: best_output_pool.address,
                zero_for_one: !output_uniswap_pool.base_is_token0,
                best_fee: best_output_pool.fee,
            },
        })
    }
}
