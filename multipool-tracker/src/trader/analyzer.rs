use colored::Colorize;

use std::{collections::HashMap, sync::Arc};

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

pub fn get_pool_fee(address: &Address) -> (Vec<u32>, String) {
    let hm: HashMap<Address, (Vec<u32>, String)> = [
        (
            "0xfc5a1a6eb076a2c7ad06ed22c90d7e710e35ad0a"
                .parse()
                .unwrap(),
            ([10000, 3000, 500].into(), "GMX".into()),
        ),
        (
            "0x3082cc23568ea640225c2467653db90e9250aaa0"
                .parse()
                .unwrap(),
            ([10000, 3000].into(), "RDNT".into()),
        ),
        (
            "0x0341c0c0ec423328621788d4854119b97f44e391"
                .parse()
                .unwrap(),
            ([10000].into(), "SILO".into()),
        ),
        (
            "0x539bde0d7dbd336b79148aa742883198bbf60342"
                .parse()
                .unwrap(),
            ([10000, 3000, 500].into(), "MAGIC".into()),
        ),
        (
            "0x51fc0f6660482ea73330e414efd7808811a57fa2"
                .parse()
                .unwrap(),
            ([3000].into(), "PREMIA".into()),
        ),
        (
            "0x0c880f6761f1af8d9aa9c466984b80dab9a8c9e8"
                .parse()
                .unwrap(),
            ([10000, 3000].into(), "PENDLE".into()),
        ),
    ]
    .into();
    hm.get(address).unwrap().to_owned()
}

pub async fn get_multipool_params<P: Middleware>(
    provider: Arc<P>,
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

    let name1 = get_pool_fee(&asset1.address).1;
    let name2 = get_pool_fee(&asset2.address).1;

    let quote_to_balance1 =
        (U256::try_from(asset1.balancing_data.quantity_to_upper_bound.abs()).unwrap() * price1)
            >> 96;
    let quote_to_balance2 =
        (U256::try_from(asset2.balancing_data.quantity_to_lower_bound.abs()).unwrap() * price2)
            >> 96;

    //let quote_to_balance1 =
    //    (U256::try_from(asset1.balancing_data.quantity_to_balance.abs()).unwrap() * price1) >> 96;
    //let quote_to_balance2 =
    //    (U256::try_from(asset2.balancing_data.quantity_to_balance.abs()).unwrap() * price2) >> 96;

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

    println!(
        "IN:  {} {:.4} -> {:.4}",
        name1, asset1.balancing_data.deviation, 1
    );
    println!(
        "Out: {} {:.4} -> {:.4}",
        name2, asset2.balancing_data.deviation, 1
    );
    println!(
        "Multipool quote in: {}",
        quote_to_use.as_u128() as f64 / 10f64.powf(18f64)
    );
    println!(
        "Multipool fee: {}",
        fee.as_i128() as f64 * 100f64 / quote_to_use.as_u128() as f64
    );

    Ok((
        U256::try_from(amounts[1].max(amounts[0]).abs()).unwrap(),
        U256::try_from(amounts[1].min(amounts[0]).abs()).unwrap(),
        fee,
    ))
}

pub async fn analyze<P: Middleware>(
    provider: Arc<P>,
    asset_in: AssetInfo,
    asset_out: AssetInfo,
    force_push: ForcePushArgs,
    weth: Address,
) -> Result<(U256, U256, (u32, u32)), String> {
    let quoter = Quoter::new(
        "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
            .parse::<Address>()
            .unwrap(),
        provider.clone(),
    );

    let (amount_of_in, amount_of_out, fees) =
        get_multipool_params(provider, asset_in.clone(), asset_out.clone(), force_push).await?;

    let mut best_fee_in = None;
    let mut strategy_input = None;
    for fee in get_pool_fee(&asset_in.address).0 {
        let strategy_input_new = quoter
            .quote_exact_output_single(weth, asset_in.address, fee, amount_of_in, U256::zero())
            .call()
            .await
            .map_err(|e| format!("{e:?}"))?;
        if strategy_input.is_none() || strategy_input_new < strategy_input.unwrap() {
            //println!("stragegy input: {}", strategy_input_new);
            best_fee_in = Some(fee);
            strategy_input = Some(strategy_input_new);
        }
    }

    let mut best_fee_out = None;
    let mut strategy_output = None;
    for fee in get_pool_fee(&asset_out.address).0 {
        let strategy_output_new = quoter
            .quote_exact_input_single(asset_out.address, weth, fee, amount_of_out, U256::zero())
            .call()
            .await
            .map_err(|e| format!("{e:?}"))?;
        if strategy_output.is_none() || strategy_output_new > strategy_output.unwrap() {
            //println!("stragegy output: {}", strategy_output_new);
            best_fee_out = Some(fee);
            strategy_output = Some(strategy_output_new);
        }
    }

    let strategy_input = strategy_input.unwrap();
    let best_fee_in = best_fee_in.unwrap();
    let strategy_output = strategy_output.unwrap();
    let best_fee_out = best_fee_out.unwrap();

    let result = strategy_output.as_u128() as f64 / strategy_input.as_u128() as f64;
    println!("MULTIPLIER: {}", result);

    let result =
        (I256::from_raw(strategy_output) - fees).as_i128() as f64 / strategy_input.as_u128() as f64;
    println!("MULTIPLIER WITH FEE: {}", result);

    //if result.gt(&1f64) {
    //    println!("{}", "Good ratio".green().bold());
    //} else {
    //    println!("{}", "Bad ratio".red().bold());
    //}
    //println!(
    //    "Required ETH: {}",
    //    strategy_input.as_u128() as f64 / 10f64.powf(18f64)
    //);
    let profit = I256::from_raw(strategy_output) - I256::from_raw(strategy_input) - fees;
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
    // println!("in: {}", asset_in.address.to_string());
    // println!("fee in: {}", get_pool_fee(&asset_in.address));
    // println!("out: {}", asset_out.address.to_string());
    // println!("fee out: {}", get_pool_fee(&asset_out.address));
    // println!("amount of in {}", amount_of_in);
    // println!(
    //     "force push ts {} price {}",
    //     force_push.clone().timestamp,
    //     force_push.clone().share_price
    // );
    // println!(
    //     "signature {}",
    //     encode(force_push.clone().signatures[0].as_ref())
    // );
    Ok((
        amount_of_in,
        amount_of_out * 9 / 10,
        (best_fee_in, best_fee_out),
    ))
}
