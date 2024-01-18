pub mod analyzer;
pub mod uniswap;

use colored::Colorize;
use tokio_postgres::Client;

use std::{sync::Arc, time::Duration};

use ethers::{prelude::*, utils::hex::decode};
use tokio::time::sleep;

use crate::{
    config::BotConfig, crypto::SignedSharePrice, multipool_storage::MultipoolStorage,
    trader::analyzer::Estimates,
};

use self::analyzer::{AssetInfo, Stats};

abigen!(TraderContract, "src/abi/trader.json");

pub async fn run(storage: MultipoolStorage, config: BotConfig, pg_client: Client) {
    let uniswap_data = config.uniswap.clone();
    loop {
        let data = storage.get_multipools_data();
        for (multipool_id, multipool) in data {
            let wallet: LocalWallet = LocalWallet::from_bytes(
                decode(std::env::var("KEY").unwrap())
                    .expect("Failed to decode")
                    .as_slice(),
            )
            .unwrap()
            .with_chain_id(42161u64);

            let client = SignerMiddleware::new(multipool.provider.clone(), wallet);
            let client = Arc::new(client);
            let trader = TraderContract::new(
                "0x8B651f5a87DE6f496a725B9F0143F88e99D15bB0"
                    .parse::<Address>()
                    .unwrap(),
                client,
            );
            let sp: SignedSharePrice =
                reqwest::get("https://api.arcanum.to/oracle/v1/signed_price?multipool_id=arbi")
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();

            let deviations = multipool
                .get_quantities_to_balance(U256::from_dec_str(&sp.share_price).unwrap(), 180)
                .unwrap()
                .into_iter();

            let missing = deviations.clone().collect::<Vec<_>>();
            let not_missing = deviations.clone().collect::<Vec<_>>();

            for (missing_address, missing_deviation) in missing.iter() {
                let missing_asset = multipool.assets.get(missing_address).unwrap();
                for (not_missing_address, not_missing_deviation) in not_missing.iter() {
                    if missing_address == not_missing_address {
                        continue;
                    }
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

                    match analyzer::analyze(
                        multipool.provider.clone(),
                        multipool.clone(),
                        &uniswap_data,
                        false,
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
                        force_push.clone(),
                        weth,
                    )
                    .await
                    {
                        Ok(Estimates::Profitable((args, stats))) => {
                            let execution = check_and_send(args, trader.clone()).await;
                            save_stats(&pg_client, multipool_id.clone(), stats, Some(execution))
                                .await;
                        }
                        Ok(Estimates::NonProfitable(stats)) => {
                            save_stats(&pg_client, multipool_id.clone(), stats, None).await;
                            continue;
                        }
                        Err(e) => {
                            println!("{e:?}");
                            continue;
                        }
                    }

                    match analyzer::analyze(
                        multipool.provider.clone(),
                        multipool.clone(),
                        &uniswap_data,
                        true,
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
                        force_push.clone(),
                        weth,
                    )
                    .await
                    {
                        Ok(Estimates::Profitable((args, stats))) => {
                            let execution = check_and_send(args, trader.clone()).await;
                            save_stats(&pg_client, multipool_id.clone(), stats, Some(execution))
                                .await;
                        }
                        Ok(Estimates::NonProfitable(stats)) => {
                            save_stats(&pg_client, multipool_id.clone(), stats, None).await;
                            continue;
                        }
                        Err(e) => {
                            println!("{e:?}");
                            continue;
                        }
                    }
                    println!("---------------------------------------");
                }
                //sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub estimated_gas: U256,
    pub estimated_profit: U256,
    pub transaction: Option<Result<TxHash, String>>,
}

pub async fn check_and_send<M: Middleware>(
    args: Args,
    contract: TraderContract<M>,
) -> Result<Execution, String> {
    let arbitrage_call = contract
        .trade(args.clone())
        .gas(args.gas_limit)
        // 0.02
        .value(20000000000000000u128);
    match arbitrage_call.call().await {
        Ok((profit, gas_used)) => {
            let gas_used = {
                let val = arbitrage_call.estimate_gas().await;
                println!("gas: {val:?}");
                val.unwrap_or(gas_used)
            };
            println!("Simlulation SUCCESS, profit: {}, gas: {}", profit, gas_used);
            // * 0.1 / 10^9
            let eth_for_gas = gas_used * U256::from(1_000_000_000_000_000_000u128)
                / U256::from(10_000_000_000u128);
            println!(
                "ETH for gas {}",
                eth_for_gas.as_u128() as f64 / 10f64.powf(18f64)
            );

            if profit > eth_for_gas {
                println!(
                    "Actual profit {}",
                    (profit - eth_for_gas).as_u128() as f64 / 10f64.powf(18f64)
                );
                let val = match arbitrage_call.send().await {
                    Ok(v) => {
                        println!("Successful trade {:?}", v);
                        Ok(Execution {
                            estimated_gas: gas_used,
                            estimated_profit: profit,
                            transaction: Some(Ok(*v)),
                        })
                    }
                    Err(e) => {
                        println!("Trade failed {:?}", e);
                        Ok(Execution {
                            estimated_gas: gas_used,
                            estimated_profit: profit,
                            transaction: Some(Err(e.to_string())),
                        })
                    }
                };
                sleep(Duration::from_millis(4000)).await;
                val
            } else {
                let profit_with_gas = format!(
                    "Profit with gas: {}",
                    (profit.as_u128() as i128 - eth_for_gas.as_u128() as i128) as f64
                        / 10f64.powf(18f64)
                );
                println!("{}", profit_with_gas.yellow().bold());
                Ok(Execution {
                    estimated_gas: gas_used,
                    estimated_profit: profit,
                    transaction: None,
                })
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
            Err(e.to_string())
        }
    }
}

pub async fn save_stats(
    pg_client: &Client,
    multipool_id: String,
    stats: Stats,
    execution: Option<Result<Execution, String>>,
) {
    let (estimation_error, estimation_gas, estimation_profit) = match execution {
        Some(e) => match e {
            Ok(v) => (
                None,
                Some(v.estimated_gas.to_string()),
                Some(v.estimated_profit.to_string()),
            ),
            Err(e) => (Some(e), None, None),
        },
        None => (None, None, None),
    };
    let r = pg_client
        .execute(
            "
        insert into trader_stats(
            multipool_id,
            asset_in_address,
            asset_out_address,
            timestamp,
            row_timestamp,

            pool_in_address,
            pool_out_address,
            strategy,

            profit_ratio,
            strategy_input,
            strategy_output,
            multipool_fee,
            multipool_amount_in,
            multipool_amount_out,

            asset_in_symbol,
            asset_out_symbol,

            multipool_asset_in_price,
            multipool_asset_out_price,

            pool_in_fee,
            pool_out_fee,

            estimation_error,
            estimated_gas,
            estimated_profit
            ) values (
                $1,$2,$3::TEXT::BIGINT,$4::TEXT::BIGINT,
                $5,$6,$7,
                $8::TEXT::NUMERIC,$9::TEXT::NUMERIC,$10::TEXT::NUMERIC,$11::TEXT::NUMERIC,
                $12::TEXT::NUMERIC,$13::TEXT::NUMERIC,
                $14,$15,$16::TEXT::NUMERIC,$17::TEXT::NUMERIC,$18::TEXT::INT,$19::TEXT::INT,
                $20,$21::TEXT::NUMERIC, $22::TEXT::NUMERIC)
          ",
            &[
                &multipool_id,
                &serde_json::to_string(&stats.asset_in_address)
                    .unwrap()
                    .trim_matches('\"'),
                &serde_json::to_string(&stats.asset_out_address)
                    .unwrap()
                    .trim_matches('\"'),
                &(stats.timestamp / (1000 * 60 * 5) * (1000 * 60 * 5)).to_string(),
                &(stats.timestamp).to_string(),
                &serde_json::to_string(&stats.pool_in_address)
                    .unwrap()
                    .trim_matches('\"'),
                &serde_json::to_string(&stats.pool_out_address)
                    .unwrap()
                    .trim_matches('\"'),
                &stats.strategy,
                &stats.profit_ratio.to_string(),
                &stats.strategy_input.to_string(),
                &stats.strategy_output.to_string(),
                &stats.multipool_fee.to_string(),
                &stats.multipool_amount_in.to_string(),
                &stats.multipool_amount_out.to_string(),
                &stats.asset_in_symbol,
                &stats.asset_out_symbol,
                &stats.multipool_asset_in_price.to_string(),
                &stats.multipool_asset_out_price.to_string(),
                &stats.pool_in_fee.to_string(),
                &stats.pool_out_fee.to_string(),
                &estimation_error,
                &estimation_gas,
                &estimation_profit,
            ],
        )
        .await;
    println!("{r:?}");
}
