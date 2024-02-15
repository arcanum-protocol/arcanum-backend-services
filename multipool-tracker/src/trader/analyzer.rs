use colored::Colorize;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ethers::prelude::*;

use crate::{
    multipool_storage::{expiry::MayBeExpired, Multipool, MultipoolAsset},
    rpc_controller::RpcRobber,
    trader::Args,
};

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

pub const RETRIES: Option<usize> = Some(3);

pub async fn get_multipool_params(
    rpc: &RpcRobber,
    multipool: &Multipool,
    uniswap: &Uniswap,
    maximize_volume: bool,
    asset1: Address,
    asset2: Address,
    force_push: ForcePushArgs,
) -> Result<(U256, U256, I256), String> {
    let price1 = multipool
        .get_price(&asset1)
        .unwrap()
        .not_older_than(180)
        .unwrap();
    let price2 = multipool
        .get_price(&asset2)
        .unwrap()
        .not_older_than(180)
        .unwrap();

    let name1 = uniswap.get_pool_fee(&asset1)?.asset_symbol;
    let name2 = uniswap.get_pool_fee(&asset2)?.asset_symbol;

    let target_deviation = I256::from(644245094) / 2;

    let to_u256 = |val: Option<MayBeExpired<I256>>, price| {
        U256::try_from(val.unwrap().not_older_than(180).unwrap().abs()).unwrap() * price >> 96
    };

    let (quote_to_balance1, quote_to_balance2) = if maximize_volume {
        (
            to_u256(
                multipool.quantity_to_deviation(&asset1, target_deviation, 180),
                price1,
            ),
            to_u256(
                multipool.quantity_to_deviation(&asset2, -target_deviation, 180),
                price2,
            ),
        )
    } else {
        (
            to_u256(
                multipool.quantity_to_deviation(&asset1, I256::zero(), 180),
                price1,
            ),
            to_u256(
                multipool.quantity_to_deviation(&asset2, I256::zero(), 180),
                price2,
            ),
        )
    };

    println!("{quote_to_balance1} {quote_to_balance2}");

    let quote_to_use = quote_to_balance1.min(quote_to_balance2);

    let amount_to_use = (quote_to_use << 96) / price1;

    println!("{name1} -> {name2}");
    let mut swap_args = vec![
        multipool::AssetArgs {
            asset_address: asset1,
            amount: I256::from_raw(amount_to_use),
        },
        multipool::AssetArgs {
            asset_address: asset2,
            amount: I256::from(-10000000i128),
        },
    ];

    swap_args.sort_by_key(|v| v.asset_address);

    let (fee, amounts): (I256, Vec<I256>) = rpc
        .aquire(
            |provider| async {
                let swap_args = swap_args.clone();
                let force_push = force_push.clone();
                multipool::MultipoolContract::new(multipool.contract_address(), provider)
                    .check_swap(force_push, swap_args, true)
                    .call()
                    .await
            },
            RETRIES,
        )
        .await
        .map_err(|e| format!("{e:?}"))?;

    let amount_of_in = U256::try_from(amounts[1].max(amounts[0]).abs()).unwrap();
    let amount_of_out = U256::try_from(amounts[1].min(amounts[0]).abs()).unwrap();

    Ok((amount_of_in, amount_of_out, fee))
}

pub enum AmountWithDirection {
    ExactInput(U256),
    ExactOutput(U256),
}

pub async fn estimate_uniswap(
    rpc: &RpcRobber,
    uniswap: &Uniswap,
    asset: Address,
    amount: AmountWithDirection,
    weth: Address,
) -> Result<(U256, Address, bool, u32), String> {
    let mut best_pool = None;
    let mut best_fee = None;
    let mut estimated = None;
    let uniswap_pool = uniswap.get_pool_fee(&asset)?;
    let zero_for_one = match amount {
        AmountWithDirection::ExactInput(_) => uniswap_pool.base_is_token0,
        AmountWithDirection::ExactOutput(_) => !uniswap_pool.base_is_token0,
    };

    for PoolInfo { fee, address } in uniswap_pool.pools {
        match amount {
            AmountWithDirection::ExactOutput(amount) => {
                let strategy_input_new = rpc
                    .aquire(
                        |provider| async {
                            Quoter::new(
                                "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
                                    .parse::<Address>()
                                    .unwrap(),
                                provider,
                            )
                            .quote_exact_output_single(weth, asset, fee, amount, U256::zero())
                            .call()
                            .await
                        },
                        RETRIES,
                    )
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                if estimated.is_none() || strategy_input_new < estimated.unwrap() {
                    best_pool = Some(address);
                    estimated = Some(strategy_input_new);
                    best_fee = Some(fee);
                }
            }
            AmountWithDirection::ExactInput(amount) => {
                let strategy_output_new = rpc
                    .aquire(
                        |provider| async {
                            Quoter::new(
                                "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
                                    .parse::<Address>()
                                    .unwrap(),
                                provider,
                            )
                            .quote_exact_input_single(asset, weth, fee, amount, U256::zero())
                            .call()
                            .await
                        },
                        RETRIES,
                    )
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                if estimated.is_none() || strategy_output_new > estimated.unwrap() {
                    best_pool = Some(address);
                    estimated = Some(strategy_output_new);
                    best_fee = Some(fee);
                }
            }
        };
    }
    Ok((
        estimated.unwrap(),
        best_pool.unwrap(),
        zero_for_one,
        best_fee.unwrap(),
    ))
}

pub enum Estimates {
    Profitable((Args, Stats)),
    NonProfitable(Stats),
}

pub struct Stats {
    pub profit_ratio: f64,
    pub strategy_input: U256,
    pub strategy_output: U256,
    pub multipool_fee: I256,
    pub asset_in_address: Address,
    pub asset_out_address: Address,
    pub pool_in_address: Address,
    pub pool_out_address: Address,
    pub multipool_amount_in: U256,
    pub multipool_amount_out: U256,
    pub strategy: String,
    pub timestamp: u128,

    pub multipool_asset_in_price: U256,
    pub multipool_asset_out_price: U256,

    pub asset_in_symbol: String,
    pub asset_out_symbol: String,

    pub pool_in_fee: u32,
    pub pool_out_fee: u32,
}

pub async fn analyze(
    rpc: &RpcRobber,
    multipool: &Multipool,
    uniswap: &Uniswap,
    maximize_volume: bool,
    asset_in: Address,
    asset_out: Address,
    force_push: ForcePushArgs,
    weth: Address,
) -> Result<Estimates, String> {
    let (amount_of_in, amount_of_out, fees) = get_multipool_params(
        rpc,
        multipool,
        uniswap,
        maximize_volume,
        asset_in.clone(),
        asset_out.clone(),
        force_push.clone(),
    )
    .await?;

    let (strategy_input, pool_in, zero_for_one_in, pool_in_fee) = estimate_uniswap(
        rpc,
        uniswap,
        asset_in.clone(),
        AmountWithDirection::ExactOutput(amount_of_in),
        weth,
    )
    .await?;

    let (strategy_output, pool_out, zero_for_one_out, pool_out_fee) = estimate_uniswap(
        rpc,
        uniswap,
        asset_out.clone(),
        AmountWithDirection::ExactInput(amount_of_out),
        weth,
    )
    .await?;

    let result =
        (I256::from_raw(strategy_output) - fees).as_i128() as f64 / strategy_input.as_u128() as f64;

    let uniswap_in_pool = uniswap.get_pool_fee(&asset_in)?;
    let uniswap_out_pool = uniswap.get_pool_fee(&asset_out)?;

    let price1 = multipool
        .get_price(&asset_in)
        .unwrap()
        .not_older_than(180)
        .unwrap();
    let price2 = multipool
        .get_price(&asset_out)
        .unwrap()
        .not_older_than(180)
        .unwrap();

    let stats = Stats {
        asset_in_symbol: uniswap_in_pool.asset_symbol,
        asset_out_symbol: uniswap_out_pool.asset_symbol,

        multipool_asset_in_price: price1,
        multipool_asset_out_price: price2,

        pool_in_fee,
        pool_out_fee,

        profit_ratio: result,
        strategy_input,
        strategy: if maximize_volume {
            "max_volume".to_string()
        } else {
            "balancing".to_string()
        },
        strategy_output,
        multipool_fee: fees,
        asset_in_address: asset_in.clone(),
        asset_out_address: asset_out.clone(),
        pool_in_address: pool_in.clone(),
        pool_out_address: pool_out.clone(),
        multipool_amount_in: amount_of_in.clone(),
        multipool_amount_out: amount_of_out.clone(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    };

    if I256::from_raw(strategy_output) - fees < I256::from_raw(strategy_input) {
        return Ok(Estimates::NonProfitable(stats));
    }

    let args = Args {
        token_in: asset_in,
        zero_for_one_in,
        token_out: asset_out,
        zero_for_one_out,

        multipool_amount_in: amount_of_in,
        multipool_amount_out: amount_of_out * 9 / 10,
        multipool_fee: 1000000000000000u128.into(),

        pool_in,
        pool_out,

        multipool: multipool.contract_address(),
        fp: crate::trader::ForcePushArgs {
            contract_address: force_push.contract_address,
            timestamp: force_push.timestamp,
            share_price: force_push.share_price,
            signatures: force_push.signatures,
        },
        gas_limit: 5000000.into(),
        weth,
    };

    Ok(Estimates::Profitable((args, stats)))
}
