use colored::Colorize;

use std::{collections::HashMap, sync::Arc, time::Duration};

use ethers::{
    prelude::*,
    utils::hex::{decode, encode},
};
use tokio::time::sleep;

use crate::{
    crypto::SignedSharePrice,
    multipool_storage::{BalancingData, MultipoolAsset, MultipoolStorage},
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

#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub address: Address,
    pub balancing_data: BalancingData,
    pub asset_data: MultipoolAsset,
}

pub fn get_pool_fee(address: &Address) -> u32 {
    let hm: HashMap<Address, u32> = [
        (
            "0xfc5a1a6eb076a2c7ad06ed22c90d7e710e35ad0a"
                .parse()
                .unwrap(),
            3000,
        ),
        (
            "0x3082cc23568ea640225c2467653db90e9250aaa0"
                .parse()
                .unwrap(),
            10000,
        ),
        (
            "0x0341c0c0ec423328621788d4854119b97f44e391"
                .parse()
                .unwrap(),
            10000,
        ),
        (
            "0x539bde0d7dbd336b79148aa742883198bbf60342"
                .parse()
                .unwrap(),
            10000,
        ),
        (
            "0x51fc0f6660482ea73330e414efd7808811a57fa2"
                .parse()
                .unwrap(),
            3000,
        ),
        (
            "0x0c880f6761f1af8d9aa9c466984b80dab9a8c9e8"
                .parse()
                .unwrap(),
            10000,
        ),
    ]
    .into();
    hm.get(address).unwrap().to_owned()
}

pub async fn analyze<P: Middleware>(
    provider: Arc<P>,
    asset_in: AssetInfo,
    asset_out: AssetInfo,
    force_push: ForcePushArgs,
    weth: Address,
) -> (U256, U256) {
    let quoter = Quoter::new(
        "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
            .parse::<Address>()
            .unwrap(),
        provider.clone(),
    );

    let mp_contract = multipool::MultipoolContract::new(
        "0x4810E5A7741ea5fdbb658eDA632ddfAc3b19e3c6"
            .parse::<Address>()
            .unwrap(),
        provider.clone(),
    );

    let in_price = asset_in.asset_data.price.not_older_than(180).unwrap();
    let out_price = asset_out.asset_data.price.not_older_than(180).unwrap();

    let (amount_of_out, amount_of_in, fees) =
        if ((asset_in.balancing_data.quantity_to_balance * in_price) >> 96)
            < ((asset_out.balancing_data.quantity_to_balance * out_price) >> 96)
        {
            let mut swap_args = vec![
                multipool::AssetArgs {
                    asset_address: asset_in.address,
                    amount: I256::from_raw(asset_in.balancing_data.quantity_to_balance),
                },
                multipool::AssetArgs {
                    asset_address: asset_out.address,
                    amount: I256::from(-10000000000000000i128),
                },
            ];
            swap_args.sort_by_key(|v| v.asset_address);

            let (fee, amounts): (I256, Vec<I256>) = mp_contract
                .check_swap(force_push.clone(), swap_args, true)
                .call()
                .await
                .unwrap();

            if asset_in.address < asset_out.address {
                (-amounts[1], amounts[0], fee)
            } else {
                (-amounts[0], amounts[1], fee)
            }
        } else {
            let mut swap_args = vec![
                multipool::AssetArgs {
                    asset_address: asset_in.address,
                    amount: I256::from(100000000000000000i128),
                },
                multipool::AssetArgs {
                    asset_address: asset_out.address,
                    amount: -I256::from_raw(asset_out.balancing_data.quantity_to_balance),
                },
            ];
            swap_args.sort_by_key(|v| v.asset_address);

            let (fee, amounts): (I256, Vec<I256>) = mp_contract
                .check_swap(force_push.clone(), swap_args, false)
                .call()
                .await
                .unwrap();

            if asset_in.address < asset_out.address {
                (-amounts[1], amounts[0], fee)
            } else {
                (-amounts[0], amounts[1], fee)
            }
        };

    let (amount_of_in, amount_of_out) = (
        U256::from_dec_str(&amount_of_in.to_string()).unwrap(),
        U256::from_dec_str(&amount_of_out.to_string()).unwrap(),
    );

    let strategy_input = quoter
        .quote_exact_output_single(
            weth,
            asset_in.address,
            get_pool_fee(&asset_in.address),
            amount_of_in,
            U256::zero(),
        )
        .call()
        .await
        .unwrap();

    let strategy_output = quoter
        .quote_exact_input_single(
            asset_out.address,
            weth,
            get_pool_fee(&asset_out.address),
            amount_of_out,
            U256::zero(),
        )
        .call()
        .await
        .unwrap();

    let result = strategy_output.as_u128() as f64 / strategy_input.as_u128() as f64;
    println!("MULTIPLIER: {}", result);

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
    (amount_of_in, amount_of_out * 9 / 10)
}
