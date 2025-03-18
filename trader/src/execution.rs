use alloy::providers::Provider;
use anyhow::{anyhow, Result};
use colored::Colorize;
use std::time::Duration;

use tokio::time::sleep;

use crate::clickhouse::{Click, TradeStats};
use crate::contracts::trader::Trader::{self, Args, Call};
use crate::contracts::TRADER_ADDRESS;
use crate::trade::UniswapChoise;
use alloy::hex::ToHexExt;
use alloy::primitives::{Address, TxHash, U256};

impl<P: Provider> UniswapChoise<P> {
    pub async fn execute(&self) -> Result<()> {
        let multipool = &self
            .trading_data
            .trading_data_with_assets
            .trading_data
            .multipool;
        let stats = TradeStats {
            trade_input: self.input.estimated,
            trade_output: self.output.estimated,

            multipool_fee: self.trading_data.fee,

            asset_in_address: self
                .trading_data
                .trading_data_with_assets
                .asset1
                .0
                .encode_hex(),
            asset_out_address: self
                .trading_data
                .trading_data_with_assets
                .asset2
                .0
                .encode_hex(),

            pool_in_address: self.input.best_pool.0.encode_hex(),
            pool_out_address: self.output.best_pool.0.encode_hex(),

            multipool_amount_in: self.trading_data.multipool_amount_in,
            multipool_amount_out: self.trading_data.multipool_amount_out,

            strategy_type: "collectCashbacks".into(),

            multipool_address: self
                .trading_data
                .trading_data_with_assets
                .trading_data
                .multipool
                .contract_address()
                .0
                .encode_hex(),

            pool_in_fee: self.input.best_fee,
            pool_out_fee: self.output.best_fee,
        };

        let args = Args {
            tokenIn: self.trading_data.swap_asset_in,
            multipoolTokenIn: self.trading_data.trading_data_with_assets.asset1,
            zeroForOneIn: !self.input.zero_for_one,
            tokenOut: self.trading_data.swap_asset_out,
            multipoolTokenOut: self.trading_data.trading_data_with_assets.asset2,
            zeroForOneOut: !self.output.zero_for_one,

            tmpAmount: self.trading_data.unwrapped_amount_in,
            multipoolFee: U256::from(10000000000000u128),

            poolIn: self.input.best_pool,
            poolOut: self.output.best_pool,

            multipool: multipool.contract_address(),
            oraclePrice: self
                .trading_data
                .trading_data_with_assets
                .trading_data
                .oracle_price
                .clone(),
            gasLimit: U256::from(4000000),
            weth: self.trading_data.trading_data_with_assets.trading_data.weth,
            // cashback: CASHBACK_VAULT,
            cashback: Address::ZERO,
            assets: vec![
                self.trading_data.trading_data_with_assets.asset1,
                self.trading_data.trading_data_with_assets.asset2,
            ],

            firstCall: Call {
                wrapper: self.trading_data.wrap_call.wrapper,
                data: self.trading_data.wrap_call.data.clone().into(),
            },

            secondCall: Call {
                wrapper: self.trading_data.unwrap_call.wrapper,
                data: self.trading_data.unwrap_call.data.clone().into(),
            },
        };

        check_and_send(
            &self.trading_data.trading_data_with_assets.trading_data.rpc,
            args,
        )
        .await
        .map_err(|e| anyhow!("{e:?}"))?;

        // insert post trade
        let click = Click::new()?;
        click.insert(stats).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub estimated_gas: U256,
    pub estimated_profit: U256,
    pub transaction: Option<Result<TxHash, String>>,
}

pub async fn check_and_send<P: Provider>(rpc: &P, args: Args) -> Result<Execution, String> {
    let contract = Trader::new(TRADER_ADDRESS, rpc);
    let tx = contract
        .trade(args.clone())
        .gas(args.gasLimit.to::<u64>())
        .gas_price(10000000)
        // 0.02
        .value(U256::from(30000000000000000u128));
    let simulate = tx.call().await;

    match simulate {
        Ok(res) => {
            let profit = res.profit;
            let gas_used = res.gasUsed;
            println!("Simlulation SUCCESS, profit: {}, gas: {}", profit, gas_used);
            // * 0.1 / 10^9
            let eth_for_gas = gas_used * U256::from(10_000_000u128);
            println!(
                "ETH for gas {}",
                eth_for_gas.to::<u128>() as f64 / 10f64.powf(18f64)
            );

            if profit > eth_for_gas {
                println!(
                    "Actual profit {}",
                    (profit - eth_for_gas).to::<u128>() as f64 / 10f64.powf(18f64)
                );
                let broadcast = tx.send().await;

                let val = match broadcast {
                    Ok(v) => {
                        println!("Successful trade {:?}", v);
                        Ok(Execution {
                            estimated_gas: gas_used,
                            estimated_profit: profit,
                            transaction: Some(Ok(v.tx_hash().to_owned())),
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
                    (profit.to::<i128>() - eth_for_gas.to::<i128>()) as f64 / 10f64.powf(18f64)
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
            // if e.as_revert() ==
            //     Some(
            //         bytes!("08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000\
            //         0000000000000000000000000000000000000000000000096e6f2070726f66697400000000000000000000000000\
            //         00000000000000000000")) {
            //     println!("Simulation FAILED, no profit");
            // } else if e.as_revert() == Some(
            //     bytes!("08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000\
            //     00000000000000000000000000000000000000000000000035354460000000000000000000000000000000000\
            //     000000000000000000000000")) {
            //     println!("Simulation FAILED, uniswap STF");
            // } else if e.as_revert() == Some(
            //     bytes!("08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000\
            //     000000000000000000000000000000000000000000000000096e6f2070726f6669740000000000000000000000\
            //     000000000000000000000000")) {
            //     println!("No profit");
            // } else if e.as_revert() == Some(
            //     bytes!("08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000\
            //     00000000000000000000000000000000000000000000002645524332303a207472616e7366657220616d6f756e74\
            //     20657863656564732062616c616e63650000000000000000000000000000000000000000000000000000")) {
            //     println!("Simulation FAILED, ERC20: transfer amount exceeds balance");

            // } else if e.as_revert() == Some(
            //     bytes!("08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000\
            //     000000000000000000000000000000000000000000002645524332303a207472616e7366657220616d6f756e7420657\
            //     863656564732062616c616e63650000000000000000000000000000000000000000000000000000")) {
            //     println!("ERC20: transfer amount exceeds balance");
            // } else if e.as_revert() == Some(bytes!("3fb8e961")) {
            //     println!("Simulation FAILED, deviation exceeds limit");
            // } else if e.as_revert() == Some(bytes!("7cb71f89")) {
            //     println!("Simulation FAILED, sleepage exceeded");
            // } else {
            //     println!("Simulation FAILED, error: {:?}", e);
            // }
            Err(e.to_string())
        }
    }
}
