pub mod analyzer;
pub mod uniswap;

use colored::Colorize;

use std::{sync::Arc, time::Duration};

use ethers::{prelude::*, utils::hex::decode};
use tokio::time::sleep;

use crate::{
    crypto::SignedSharePrice, multipool_storage::MultipoolStorage, trader::analyzer::get_pool_fee,
};

use self::analyzer::AssetInfo;

abigen!(TraderContract, "src/abi/trader.json");

pub async fn run(storage: MultipoolStorage) {
    loop {
        let data = storage.get_multipools_data();
        for (_, multipool) in data {
            let sp: SignedSharePrice =
                reqwest::get("https://api.arcanum.to/oracle/v1/signed_price?multipool_id=arbi")
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();

            let deviations = multipool
                .get_quantities_to_balance(U256::from_dec_str(&sp.share_price).unwrap(), 180)
                .unwrap();

            let missing = deviations
                .clone()
                .into_iter()
                .filter(|(_, data)| data.is_missing)
                .collect::<Vec<_>>();

            let not_missing = deviations
                .clone()
                .into_iter()
                .filter(|(_, data)| !data.is_missing)
                .collect::<Vec<_>>();

            for (missing_address, missing_deviation) in missing.iter() {
                let missing_asset = multipool.assets.get(missing_address).unwrap();
                for (not_missing_address, not_missing_deviation) in not_missing.iter() {
                    let not_missing_asset = multipool.assets.get(not_missing_address).unwrap();

                    let weth: Address = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"
                        .parse()
                        .unwrap();

                    let force_push = analyzer::multipool::ForcePushArgs {
                        contract_address: sp.contract_address,
                        timestamp: sp.timestamp.parse().unwrap(),
                        share_price: sp.share_price.parse().unwrap(),
                        signatures: vec![sp.signature.parse().unwrap()],
                    };

                    let (amount_of_in, amount_of_out) = analyzer::analyze(
                        multipool.provider.clone(),
                        AssetInfo {
                            address: missing_address.to_owned(),
                            balancing_data: missing_deviation.to_owned(),
                            asset_data: missing_asset.to_owned(),
                        },
                        AssetInfo {
                            address: not_missing_address.to_owned(),
                            balancing_data: not_missing_deviation.to_owned(),
                            asset_data: not_missing_asset.to_owned(),
                        },
                        force_push,
                        weth,
                    )
                    .await;

                    let multipool_fee: U256 = 1000000000000000u128.into();
                    let args = Args {
                        token_in: *missing_address,
                        token_out: *not_missing_address,
                        amount_of_in,

                        swap_router: "0xE592427A0AEce92De3Edee1F18E0157C05861564"
                            .parse()
                            .unwrap(),

                        multipool_fee,

                        out_amount: amount_of_out.into(),

                        fee_in: get_pool_fee(missing_address),
                        fee_out: get_pool_fee(not_missing_address),

                        approve_in: true,
                        approve_out: true,

                        multipool: multipool.contract_address,
                        fp: ForcePushArgs {
                            contract_address: sp.contract_address,
                            timestamp: sp.timestamp.parse().unwrap(),
                            share_price: sp.share_price.parse().unwrap(),
                            signatures: vec![sp.signature.parse().unwrap()],
                        },
                        gas_limit: 5000000.into(),
                        weth,
                    };

                    let wallet: LocalWallet = LocalWallet::from_bytes(
                        decode(std::env::var("KEY").unwrap())
                            .expect("Failed to decode")
                            .as_slice(),
                    )
                    .unwrap()
                    .with_chain_id(42161u64);

                    let client = SignerMiddleware::new(multipool.provider.clone(), wallet);
                    let client = Arc::new(client);

                    check_and_send(
                        args,
                        TraderContract::new(
                            "0x25497ea231c3e355ddC868eDa4E9A08b3e4CeB62"
                                .parse::<Address>()
                                .unwrap(),
                            client,
                        ),
                    )
                    .await;
                    println!("---------------------------------------");
                }
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

pub async fn check_and_send<M: Middleware>(args: Args, contract: TraderContract<M>) {
    let arbitrage_call = contract
        .trade(args.clone())
        .gas(args.gas_limit)
        // 0.02
        .value(20000000000000000u128);
    match arbitrage_call.call().await {
        Ok((profit, gas_used)) => {
            println!("Simlulation SUCCESS, profit: {}, gas: {}", profit, gas_used);
            // * 0.1 / 10^9
            let eth_for_gas = gas_used * U256::from(1_000_000_000_000_000_000u128)
                / U256::from(10_000_000_000u128);
            println!("ETH for gas {}", eth_for_gas.as_u128() as f64 / 10f64.powf(18f64));
            if profit > eth_for_gas {
                println!(
                    "Actual profit {}",
                    (profit - eth_for_gas).as_u128() as f64 / 10f64.powf(18f64)
                );
                match arbitrage_call.send().await {
                    Ok(v) => {
                        println!("Successful trade {:?}", v);
                    }
                    Err(e) => {
                        println!("Trade failed {:?}", e);
                    }
                }
                sleep(Duration::from_millis(4000)).await;
            } else {
                let profit_with_gas = 
                    format!("Profit with gas: {}", 
                            (profit.as_u128() as i128 - eth_for_gas.as_u128() as i128) 
                            as f64 / 10f64.powf(18f64));
                println!("{}", profit_with_gas.yellow().bold());
            }
        }

        Err(e) => {
            if e.as_revert() ==
                Some(
                    &"0x08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000\
                    0000000000000000000000000000000000000000000000096e6f2070726f66697400000000000000000000000000\
                    00000000000000000000".parse::<Bytes>().unwrap()) {
                println!("Simulation FAILED, no profit");
            } else if e.as_revert() == Some(
                &"0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000\
                00000000000000000000000000000000000000000000000035354460000000000000000000000000000000000\
                000000000000000000000000".parse::<Bytes>().unwrap()) {
                println!("Simulation FAILED, uniswap STF");
            } else if e.as_revert() == Some(
                &"0x08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000\
                00000000000000000000000000000000000000000000002645524332303a207472616e7366657220616d6f756e74\
                20657863656564732062616c616e63650000000000000000000000000000000000000000000000000000"
                .parse::<Bytes>().unwrap()) {
                println!("Simulation FAILED, ERC20: transfer amount exceeds balance");

            } else if e.as_revert() == Some(&"0x3fb8e961".parse::<Bytes>().unwrap()) {
                println!("Simulation FAILED, deviation exceeds limit");
            } else if e.as_revert() == Some(&"0x7cb71f89".parse::<Bytes>().unwrap()) {
                println!("Simulation FAILED, sleepage exceeded");
            } else {
                println!("Simulation FAILED, error: {:?}", e);
            }
        }
    }
}
