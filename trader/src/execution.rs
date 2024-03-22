use anyhow::{anyhow, Result};
use colored::Colorize;
use rpc_controller::RpcRobber;

use std::{sync::Arc, time::Duration};

use ethers::{prelude::*, utils::hex::decode};
use tokio::time::sleep;

use crate::{trade::UniswapChoise, uniswap::RETRIES};

abigen!(TraderContract, "../core/storage/src/abi/trader.json");

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

    pub pool_in_fee: u32,
    pub pool_out_fee: u32,
}

impl<'a> UniswapChoise<'a> {
    pub async fn execute(&self) -> Result<()> {
        if I256::from_raw(self.output.estimated) - self.trading_data.fee
            < I256::from_raw(self.input.estimated)
        {
            return Ok(());
        }

        let multipool = &self
            .trading_data
            .trading_data_with_assets
            .trading_data
            .multipool;

        let args = Args {
            token_in: self.trading_data.trading_data_with_assets.asset1,
            zero_for_one_in: self.input.zero_for_one,
            token_out: self.trading_data.trading_data_with_assets.asset2,
            zero_for_one_out: self.output.zero_for_one,

            multipool_amount_in: self.trading_data.amount_in,
            multipool_amount_out: self.trading_data.amount_out * 9 / 10,
            multipool_fee: 1000000000000000u128.into(),

            pool_in: self.input.best_pool,
            pool_out: self.output.best_pool,

            multipool: multipool.contract_address(),
            fp: self
                .trading_data
                .trading_data_with_assets
                .trading_data
                .force_push
                .clone(),
            gas_limit: 5000000.into(),
            weth: self.trading_data.trading_data_with_assets.trading_data.weth,
        };
        let wallet: LocalWallet = LocalWallet::from_bytes(
            decode(std::env::var("TRADER_KEY").unwrap())
                .expect("Failed to decode")
                .as_slice(),
        )
        .unwrap()
        .with_chain_id(
            self.trading_data
                .trading_data_with_assets
                .trading_data
                .rpc
                .chain_id,
        );
        check_and_send(
            &self.trading_data.trading_data_with_assets.trading_data.rpc,
            args,
            wallet,
        )
        .await
        .map_err(|e| anyhow!("{e:?}"))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub estimated_gas: U256,
    pub estimated_profit: U256,
    pub transaction: Option<Result<TxHash, String>>,
}

pub async fn check_and_send(
    rpc: &RpcRobber,
    args: Args,
    wallet: LocalWallet,
) -> Result<Execution, String> {
    let simulate = rpc
        .aquire(
            |provider, _| async {
                let client = SignerMiddleware::new(provider, wallet.clone());
                let client = Arc::new(client);
                TraderContract::new(
                    "0x8B651f5a87DE6f496a725B9F0143F88e99D15bB0"
                        .parse::<Address>()
                        .unwrap(),
                    client,
                )
                .trade(args.clone())
                .gas(args.gas_limit)
                // 0.02
                .value(20000000000000000u128)
                .call()
                .await
            },
            RETRIES,
        )
        .await;

    log::info!("{:?}", args);
    match simulate {
        Ok((profit, gas_used)) => {
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

                let broadcast = rpc
                    .aquire(
                        |provider, _| async {
                            let client = SignerMiddleware::new(provider, wallet.clone());
                            let client = Arc::new(client);
                            TraderContract::new(
                                "0x8B651f5a87DE6f496a725B9F0143F88e99D15bB0"
                                    .parse::<Address>()
                                    .unwrap(),
                                client,
                            )
                            .trade(args.clone())
                            .gas(args.gas_limit)
                            // 0.02
                            .value(20000000000000000u128)
                            .send()
                            .await
                            .map(|v| v.to_owned())
                        },
                        RETRIES,
                    )
                    .await;

                let val = match broadcast {
                    Ok(v) => {
                        println!("Successful trade {:?}", v);
                        Ok(Execution {
                            estimated_gas: gas_used,
                            estimated_profit: profit,
                            transaction: Some(Ok(v)),
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

//pub async fn save_stats(
//    pg_client: &Client,
//    multipool_id: String,
//    stats: Stats,
//    execution: Option<Result<Execution, String>>,
//) {
//    let (estimation_error, estimation_gas, estimation_profit) = match execution {
//        Some(e) => match e {
//            Ok(v) => (
//                None,
//                Some(v.estimated_gas.to_string()),
//                Some(v.estimated_profit.to_string()),
//            ),
//            Err(e) => (Some(e), None, None),
//        },
//        None => (None, None, None),
//    };
//    let r = pg_client
//        .execute(
//            "
//        insert into trader_stats(
//            multipool_id,
//            asset_in_address,
//            asset_out_address,
//            timestamp,
//            row_timestamp,
//
//            pool_in_address,
//            pool_out_address,
//            strategy,
//
//            profit_ratio,
//            strategy_input,
//            strategy_output,
//            multipool_fee,
//            multipool_amount_in,
//            multipool_amount_out,
//
//            asset_in_symbol,
//            asset_out_symbol,
//
//            multipool_asset_in_price,
//            multipool_asset_out_price,
//
//            pool_in_fee,
//            pool_out_fee,
//
//            estimation_error,
//            estimated_gas,
//            estimated_profit
//            ) values (
//                $23, $1,$2,$3::TEXT::BIGINT,$4::TEXT::BIGINT,
//                $5,$6,$7,
//                $8::TEXT::NUMERIC,$9::TEXT::NUMERIC,$10::TEXT::NUMERIC,$11::TEXT::NUMERIC,
//                $12::TEXT::NUMERIC,$13::TEXT::NUMERIC,
//                $14,$15,$16::TEXT::NUMERIC,$17::TEXT::NUMERIC,$18::TEXT::INT,$19::TEXT::INT,
//                $20,$21::TEXT::NUMERIC, $22::TEXT::NUMERIC)
//          ",
//            &[
//                &serde_json::to_string(&stats.asset_in_address)
//                    .unwrap()
//                    .trim_matches('\"'),
//                &serde_json::to_string(&stats.asset_out_address)
//                    .unwrap()
//                    .trim_matches('\"'),
//                &(stats.timestamp / (1000 * 60 * 5) * (1000 * 60 * 5)).to_string(),
//                &(stats.timestamp).to_string(),
//                &serde_json::to_string(&stats.pool_in_address)
//                    .unwrap()
//                    .trim_matches('\"'),
//                &serde_json::to_string(&stats.pool_out_address)
//                    .unwrap()
//                    .trim_matches('\"'),
//                &stats.strategy,
//                &stats.profit_ratio.to_string(),
//                &stats.strategy_input.to_string(),
//                &stats.strategy_output.to_string(),
//                &stats.multipool_fee.to_string(),
//                &stats.multipool_amount_in.to_string(),
//                &stats.multipool_amount_out.to_string(),
//                &stats.asset_in_symbol,
//                &stats.asset_out_symbol,
//                &stats.multipool_asset_in_price.to_string(),
//                &stats.multipool_asset_out_price.to_string(),
//                &stats.pool_in_fee.to_string(),
//                &stats.pool_out_fee.to_string(),
//                &estimation_error,
//                &estimation_gas,
//                &estimation_profit,
//                &multipool_id,
//            ],
//        )
//        .await;
//    println!("{r:?}");
//}
