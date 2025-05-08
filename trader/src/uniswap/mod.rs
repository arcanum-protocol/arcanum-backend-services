use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U160};
use alloy::providers::Provider;
use anyhow::{anyhow, Result};
use compute_address::{compute_pool_address, FeeAmount, FACTORY_ADDRESS};

use crate::contracts::IQuoterV2::{QuoteExactInputSingleParams, QuoteExactOutputSingleParams};
use crate::contracts::{Quoter, MULTICALL_ADDRESS, QUOTERV2_ADDRESS, WETH_ADDRESS};
use crate::trade::{MultipoolChoise, SwapOutcome, UniswapChoise};
use alloy::primitives::aliases::U256;
use alloy::providers::Failure;
use alloy::providers::MulticallBuilder;
use alloy::providers::{CallItem, MulticallItem};

mod compute_address;

#[derive(Debug)]
pub struct PoolSwapData {
    address: Address,
    fee: u32,
    base_is_token0: bool,
}

pub enum AmountWithDirection {
    ExactInput(U256),
    ExactOutput(U256),
}

impl<P: Provider> MultipoolChoise<P> {
    pub async fn estimate_uniswap(self) -> Result<UniswapChoise<P>> {
        let asset1 = self.swap_asset_in;
        let asset2 = self.swap_asset_out;
        let rpc = &self.trading_data_with_assets.trading_data.rpc;
        let q = Quoter::new(QUOTERV2_ADDRESS, rpc);
        let (in_pools, in_calls): (
            Vec<PoolSwapData>,
            Vec<CallItem<Quoter::quoteExactOutputSingleCall>>,
        ) = FeeAmount::iter()
            .map(|fee| {
                let input_uniswap_pool = compute_pool_address(
                    FACTORY_ADDRESS,
                    WETH_ADDRESS,
                    self.swap_asset_in,
                    fee,
                    None,
                );
                let pool = PoolSwapData {
                    address: input_uniswap_pool,
                    fee: fee.into(),
                    base_is_token0: WETH_ADDRESS < self.swap_asset_in,
                };
                let call = q.quoteExactOutputSingle(QuoteExactOutputSingleParams {
                    tokenIn: WETH_ADDRESS,
                    tokenOut: asset1,
                    amount: self.unwrapped_amount_in,
                    fee: fee.into(),
                    sqrtPriceLimitX96: U160::ZERO,
                });
                let target = call.target();
                let input = call.input();
                (
                    pool,
                    CallItem::<Quoter::quoteExactOutputSingleCall>::new(target, input),
                )
            })
            .unzip();
        let (out_pools, out_calls): (
            Vec<PoolSwapData>,
            Vec<CallItem<Quoter::quoteExactOutputSingleCall>>,
        ) = FeeAmount::iter()
            .map(|fee| {
                let output_uniswap_pool = compute_pool_address(
                    FACTORY_ADDRESS,
                    self.swap_asset_out,
                    WETH_ADDRESS,
                    fee,
                    None,
                );

                let pool = PoolSwapData {
                    address: output_uniswap_pool,
                    fee: fee.into(),
                    base_is_token0: WETH_ADDRESS < self.swap_asset_out,
                };
                let call = q.quoteExactInputSingle(QuoteExactInputSingleParams {
                    tokenIn: asset2,
                    tokenOut: WETH_ADDRESS,
                    amountIn: self.unwrapped_amount_out,
                    fee: fee.into(),
                    sqrtPriceLimitX96: U160::ZERO,
                });
                let target = call.target();
                let input = call.input();
                (
                    pool,
                    CallItem::<Quoter::quoteExactOutputSingleCall>::new(target, input),
                )
            })
            .unzip();

        let result = MulticallBuilder::new_dynamic(rpc)
            .extend_calls(in_calls)
            .extend_calls(out_calls)
            .try_aggregate(false)
            .await?
            .into_iter()
            .map(|r| r.map(|v| v.amountIn))
            .collect::<Vec<Result<U256, Failure>>>();
        println!("Result ------ {:?}", result);

        let (best_input_pool, input_value) = in_pools
            .iter()
            .zip(result.iter().take(4))
            .filter_map(|(a, v)| Some(a).zip(v.clone().ok()))
            .min_by(|x, y| x.1.cmp(&y.1))
            .ok_or(anyhow!("Pools not found"))?;

        let (best_output_pool, output_value) = out_pools
            .iter()
            .zip(result.iter().skip(4))
            .filter_map(|(a, v)| Some(a).zip(v.clone().ok()))
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

fn decode_mc_quoter_result(
    call_result: &Result<DynSolValue, alloy::primitives::Bytes>,
) -> Option<U256> {
    if call_result.is_ok() {
        match call_result.as_ref().unwrap() {
            DynSolValue::Tuple(decoded) => Some(decoded[0].clone().as_uint().unwrap().0),
            _ => unreachable!(),
        }
    } else {
        None
    }
}
