use serde::{Deserialize, Serialize};

use std::time::{SystemTime, UNIX_EPOCH};

use ethers::prelude::*;

use crate::{
    multipool::{expiry::MayBeExpired, Multipool},
    rpc_controller::RpcRobber,
    trader::Args,
};

use self::multipool::ForcePushArgs;

pub enum Estimates {
    Profitable((Args, Stats)),
    NonProfitable(Stats),
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

    Ok(Estimates::Profitable((args, stats)))
}
