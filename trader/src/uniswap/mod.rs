use alloy::dyn_abi::DynSolValue;
use alloy::primitives::Address;
use anyhow::{anyhow, Result};
use compute_address::{compute_pool_address, FeeAmount, FACTORY_ADDRESS};

use crate::contracts::{Quoter, WETH_ADDRESS};
use crate::trade::{MultipoolChoise, SwapOutcome, UniswapChoise};
use alloy::primitives::{address, aliases::U256};
use alloy_multicall::Multicall;

mod compute_address;

pub struct PoolSwapData {
    address: Address,
    fee: u32,
    base_is_token0: bool,
}

pub enum AmountWithDirection {
    ExactInput(U256),
    ExactOutput(U256),
}

impl MultipoolChoise {
    pub async fn estimate_uniswap(self) -> Result<UniswapChoise> {
        let asset1 = self.swap_asset_in;
        let asset2 = self.swap_asset_out;
        let rpc = &self.trading_data_with_assets.trading_data.rpc;
        let f = Quoter::abi::functions();
        let inp = &f.get("quoteExactInputSingle").unwrap()[0];
        let out = &f.get("quoteExactOutputSingle").unwrap()[0];
        let mut mc = Multicall::new(rpc, address!("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"));
        let mut pools = Vec::with_capacity(8);
        for fee in FeeAmount::iter() {
            let input_uniswap_pool =
                compute_pool_address(FACTORY_ADDRESS, WETH_ADDRESS, self.swap_asset_in, fee, None);
            pools.push(PoolSwapData {
                address: input_uniswap_pool,
                fee: fee.into(),
                base_is_token0: WETH_ADDRESS < self.swap_asset_in,
            });
            mc.add_call(
                address!("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"),
                &inp,
                &[
                    DynSolValue::Address(WETH_ADDRESS),
                    DynSolValue::Address(asset1),
                    DynSolValue::Uint(fee.to_val(), 24),
                    DynSolValue::Uint(self.unwrapped_amount_in, 256),
                    DynSolValue::Uint(U256::ZERO, 160),
                ],
                true,
            );
        }

        // two equals iterations to pack results in order like (4 inputs -- 4 outputs)
        // make another iteration of 4 is simplier than try to partition the Vec<Result<DynValue>> by x & 1 predicate
        for fee in FeeAmount::iter() {
            let output_uniswap_pool = compute_pool_address(
                FACTORY_ADDRESS,
                self.swap_asset_out,
                WETH_ADDRESS,
                fee,
                None,
            );
            pools.push(PoolSwapData {
                address: output_uniswap_pool,
                fee: fee.into(),
                base_is_token0: WETH_ADDRESS < self.swap_asset_in,
            });
            mc.add_call(
                address!("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"),
                &out,
                &[
                    DynSolValue::Address(asset2),
                    DynSolValue::Address(WETH_ADDRESS),
                    DynSolValue::Uint(fee.to_val(), 24),
                    DynSolValue::Uint(self.unwrapped_amount_out, 256),
                    DynSolValue::Uint(U256::ZERO, 160),
                ],
                true,
            );
        }

        // let result: Vec<U256> = mc.call().await;
        let result = mc.call().await?;
        let (best_input_pool, input_value) = pools
            .iter()
            .zip(result.iter().take(4))
            .filter_map(|(a, v)| {
                if v.is_ok() {
                    // TODO! get rid of clones
                    Some((a, v.clone().unwrap().as_uint().unwrap().0))
                } else {
                    None
                }
            })
            .min_by(|x, y| x.1.cmp(&y.1))
            .ok_or(anyhow!("Pools not found"))?;

        let (best_output_pool, output_value) = pools
            .iter()
            .zip(result.iter().skip(4))
            .filter_map(|(a, v)| {
                if v.is_ok() {
                    // TODO! get rid of clones
                    Some((a, v.clone().unwrap().as_uint().unwrap().0))
                } else {
                    None
                }
            })
            .min_by(|x, y| x.1.cmp(&y.1))
            .ok_or(anyhow!("Pools not found"))?;

        Ok(UniswapChoise {
            trading_data: self,
            input: SwapOutcome {
                estimated: input_value,
                best_pool: best_input_pool.address,
                zero_for_one: best_input_pool.base_is_token0,
                best_fee: best_input_pool.fee,
            },
            output: SwapOutcome {
                estimated: output_value,
                best_pool: best_output_pool.address,
                zero_for_one: !best_output_pool.base_is_token0,
                best_fee: best_output_pool.fee,
            },
        })
    }
}
