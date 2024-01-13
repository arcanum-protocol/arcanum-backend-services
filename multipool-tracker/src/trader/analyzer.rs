use colored::Colorize;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use ethers::prelude::*;

use crate::multipool_storage::{BalancingData, MultipoolAsset};

use self::multipool::ForcePushArgs;

pub mod multipool {
    use super::abigen;
    abigen!(MultipoolContract, "src/abi/multipool.json");
}

abigen!(
    UniswapPool,
    r#"[
        function slot0() external view returns (uint160,int24,uint16,uint16,uint16,uint8,bool)
        function observe(uint32[] secondsAgos) external view returns (int56[],uint160[])
    ]"#,
);

abigen!(
    Quoter,
    r#"[
        function quoteExactInputSingle(address tokenIn,address tokenOut,uint24 fee,uint256 amountIn,uint160 sqrtPriceLimitX96) external returns (uint256 amountOut)
        function quoteExactOutputSingle(address tokenIn,address tokenOut,uint24 fee,uint256 amountOut,uint160 sqrtPriceLimitX96) external returns (uint256 amountIn)
    ]"#,
);

#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub address: Address,
    pub balancing_data: BalancingData,
    pub asset_data: MultipoolAsset,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoolInfo {
    pub fee: u32,
    pub address: Address,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AssetPools {
    pub address: Address,
    pub asset_symbol: String,
    pub base_is_token0: bool,
    pub pools: Vec<PoolInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Uniswap {
    pub eth_pools: Vec<AssetPools>,
}

impl Uniswap {
    fn get_pool_fee(&self, address: &Address) -> Result<AssetPools, String> {
        self.eth_pools
            .iter()
            .find(|a| a.address == *address)
            .map(ToOwned::to_owned)
            .ok_or("pool not found".into())
    }
}

pub async fn get_multipool_params<P: Middleware>(
    provider: Arc<P>,
    uniswap: &Uniswap,
    maximize_volume: bool,
    asset1: AssetInfo,
    asset2: AssetInfo,
    force_push: ForcePushArgs,
) -> Result<(U256, U256, I256), String> {
    let mp_contract = multipool::MultipoolContract::new(
        "0x4810E5A7741ea5fdbb658eDA632ddfAc3b19e3c6"
            .parse::<Address>()
            .unwrap(),
        provider.clone(),
    );

    let price1 = asset1.asset_data.price.not_older_than(180).unwrap();
    let price2 = asset2.asset_data.price.not_older_than(180).unwrap();

    let name1 = uniswap.get_pool_fee(&asset1.address)?.asset_symbol;
    let name2 = uniswap.get_pool_fee(&asset2.address)?.asset_symbol;

    let (quote_to_balance1, quote_to_balance2) = if maximize_volume {
        (
            (U256::try_from(asset1.balancing_data.quantity_to_upper_bound.abs()).unwrap() * price1)
                >> 96,
            (U256::try_from(asset2.balancing_data.quantity_to_lower_bound.abs()).unwrap() * price2)
                >> 96,
        )
    } else {
        (
            (U256::try_from(asset1.balancing_data.quantity_to_balance.abs()).unwrap() * price1)
                >> 96,
            (U256::try_from(asset2.balancing_data.quantity_to_balance.abs()).unwrap() * price2)
                >> 96,
        )
    };

    let quote_to_use = quote_to_balance1.min(quote_to_balance2);

    let amount_to_use = (quote_to_use << 96) / price1;

    let mut swap_args = vec![
        multipool::AssetArgs {
            asset_address: asset1.address,
            amount: I256::from_raw(amount_to_use),
        },
        multipool::AssetArgs {
            asset_address: asset2.address,
            amount: I256::from(-10000000i128),
        },
    ];

    swap_args.sort_by_key(|v| v.asset_address);

    let (fee, amounts): (I256, Vec<I256>) = mp_contract
        .check_swap(force_push.clone(), swap_args, true)
        .call()
        .await
        .map_err(|e| format!("{e:?}"))?;

    let amount_of_in = U256::try_from(amounts[1].max(amounts[0]).abs()).unwrap();
    let amount_of_out = U256::try_from(amounts[1].min(amounts[0]).abs()).unwrap();

    println!("{name1} -> {name2}");
    // println!(
    //     "IN:  {} {:.4}, {:.4}",
    //     name1,
    //     asset1.balancing_data.deviation,
    //     amount_of_in * price1 >> 96
    // );
    // println!(
    //     "Out: {} {:.4}, {:.4}",
    //     name2,
    //     asset2.balancing_data.deviation,
    //     amount_of_out * price2 >> 96
    // );

    // println!(
    //     "DIFF: {}",
    //     (amount_of_in * price1 >> 96).abs_diff(amount_of_out * price2 >> 96)
    // );
    // println!(
    //     "Multipool quote in: {}",
    //     quote_to_use.as_u128() as f64 / 10f64.powf(18f64)
    // );
    // println!(
    //     "Multipool fee: {}",
    //     fee.as_i128() as f64 * 100f64 / quote_to_use.as_u128() as f64
    // );

    Ok((amount_of_in, amount_of_out, fee))
}

pub enum AmountWithDirection {
    ExactInput(U256),
    ExactOutput(U256),
}

pub async fn estimate_uniswap<P: Middleware>(
    provider: Arc<P>,
    uniswap: &Uniswap,
    asset: AssetInfo,
    amount: AmountWithDirection,
    weth: Address,
) -> Result<(U256, Address, bool), String> {
    let quoter = Quoter::new(
        "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
            .parse::<Address>()
            .unwrap(),
        provider.clone(),
    );

    let mut best_pool = None;
    let mut estimated = None;
    let uniswap_pool = uniswap.get_pool_fee(&asset.address)?;
    let zero_for_one = match amount {
        AmountWithDirection::ExactInput(_) => uniswap_pool.base_is_token0,
        AmountWithDirection::ExactOutput(_) => !uniswap_pool.base_is_token0,
    };

    for PoolInfo { fee, address } in uniswap_pool.pools {
        match amount {
            AmountWithDirection::ExactOutput(amount) => {
                let strategy_input_new = quoter
                    .quote_exact_output_single(weth, asset.address, fee, amount, U256::zero())
                    .call()
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                if estimated.is_none() || strategy_input_new < estimated.unwrap() {
                    best_pool = Some(address);
                    estimated = Some(strategy_input_new);
                }
            }
            AmountWithDirection::ExactInput(amount) => {
                let strategy_output_new = quoter
                    .quote_exact_input_single(asset.address, weth, fee, amount, U256::zero())
                    .call()
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                if estimated.is_none() || strategy_output_new > estimated.unwrap() {
                    best_pool = Some(address);
                    estimated = Some(strategy_output_new);
                }
            }
        };
    }
    Ok((estimated.unwrap(), best_pool.unwrap(), zero_for_one))
}

pub async fn analyze<P: Middleware>(
    provider: Arc<P>,
    uniswap: &Uniswap,
    maximize_volume: bool,
    asset_in: AssetInfo,
    asset_out: AssetInfo,
    force_push: ForcePushArgs,
    weth: Address,
) -> Result<(U256, U256, ((Address, bool), (Address, bool))), String> {
    let (amount_of_in, amount_of_out, fees) = get_multipool_params(
        provider.clone(),
        uniswap,
        maximize_volume,
        asset_in.clone(),
        asset_out.clone(),
        force_push,
    )
    .await?;

    let (strategy_input, pool_in, zero_for_one_in) = estimate_uniswap(
        provider.clone(),
        uniswap,
        asset_in,
        AmountWithDirection::ExactOutput(amount_of_in),
        weth,
    )
    .await?;

    let (strategy_output, pool_out, zero_for_one_out) = estimate_uniswap(
        provider.clone(),
        uniswap,
        asset_out,
        AmountWithDirection::ExactInput(amount_of_out),
        weth,
    )
    .await?;

    let result = strategy_output.as_u128() as f64 / strategy_input.as_u128() as f64;
    println!(
        "MULTIPLIER: {}",
        if strategy_output < strategy_input {
            result.to_string().red().bold()
        } else {
            result.to_string().green().bold()
        }
    );

    let result =
        (I256::from_raw(strategy_output) - fees).as_i128() as f64 / strategy_input.as_u128() as f64;
    println!("MULTIPLIER WITH FEE: {}", result);

    let profit = I256::from_raw(strategy_output) - I256::from_raw(strategy_input) - fees;
    if profit.is_negative() {
        return Err("Non profitable".to_string());
    }
    let profit_string = format!(
        "Profit: {}{}",
        if profit.is_positive() { "+" } else { "-" },
        profit.abs().as_u128() as f64 / 10f64.powf(18f64)
    );
    println!(
        "{}",
        if profit.is_positive() {
            profit_string.green().bold()
        } else {
            profit_string.red().bold()
        }
    );
    Ok((
        amount_of_in,
        amount_of_out * 9 / 10,
        ((pool_in, zero_for_one_in), (pool_out, zero_for_one_out)),
    ))
}
