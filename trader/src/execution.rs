use alloy::providers::Provider;
use anyhow::{anyhow, Result};
use colored::Colorize;
use std::time::Duration;

use crate::clickhouse::{Click, TradeStats};
use crate::contracts::trader::Trader::{self, Args, Call};
use crate::contracts::TRADER_ADDRESS;
use crate::trade::UniswapChoise;
use alloy::hex::ToHexExt;
use alloy::primitives::{Address, TxHash, U256};
use std::sync::Arc;
use tokio::time::sleep;

impl<P: Provider> UniswapChoise<P> {
    pub async fn execute(&self, click: Arc<Click>) -> Result<()> {
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
                .address
                .0
                .encode_hex(),

            pool_in_fee: self.input.best_fee,
            pool_out_fee: self.output.best_fee,
        };

        let args = Args {
            tokenIn: self.trading_data.swap_asset_in,
            multipoolTokenIn: self.trading_data.trading_data_with_assets.asset1,
            zeroForOneIn: self.input.zero_for_one,
            tokenOut: self.trading_data.swap_asset_out,
            multipoolTokenOut: self.trading_data.trading_data_with_assets.asset2,
            zeroForOneOut: self.output.zero_for_one,

            tmpAmount: self.trading_data.unwrapped_amount_in,
            multipoolFee: U256::from(10000000000000u128),

            poolIn: self.input.best_pool,
            poolOut: self.output.best_pool,

            multipool: multipool.address,
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
            self.input.estimated
        )
        .await
        .map_err(|e| anyhow!("value: {} {e:?}", self.input.estimated))?;
        // insert post trade
        click.insert(stats).await.map_err(|e| anyhow!("clickhouse error {e}"))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub estimated_gas: U256,
    pub estimated_profit: U256,
    pub transaction: Option<Result<TxHash, String>>,
}

pub async fn check_and_send<P: Provider>(rpc: &P, args: Args, estimated_input: U256) -> Result<Execution, String> {
    let contract = Trader::new(TRADER_ADDRESS, rpc);
    let tx = contract
        .trade(args.clone())
        .gas(args.gasLimit.to::<u64>())
        // .gas_price(10000000)
        // 0.3
        // 76.14526
        .value(estimated_input + U256::from(1000000000000000_u128));
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
            Err(format!("simulation failure: {} \n args: {:?}", e.to_string(), args))
        }
    }
}


//  assets: {0xe0590015a873bf326bd645c3e1266d4db41c4e6b: MpAsset { address: 0xe0590015a873bf326bd645c3e1266d4db41c4e6b, quantity: 9175102860042797953, price: 1561499552145916762214632838, target_share: 10 }, 0xfe140e1dce99be9f4f15d657cd9b7bf622270c50: MpAsset { address: 0xfe140e1dce99be9f4f15d657cd9b7bf622270c50, quantity: 22596109291089760153, price: 143243544969976588358540309, target_share: 10 }, 0x0f0bdebf0f83cd1ee3974779bcb7315f9808c714: MpAsset { address: 0x0f0bdebf0f83cd1ee3974779bcb7315f9808c714, quantity: 7262909028320197317, price: 271328483201903673084347247709, target_share: 10 }, 0xaeef2f6b429cb59c9b2d7bb2141ada993e8571c3: MpAsset { address: 0xaeef2f6b429cb59c9b2d7bb2141ada993e8571c3, quantity: 845558493087915468, price: 79757422070784428608687450987, target_share: 10 }, 0xb2f82d0f38dc453d596ad40a37799446cc89274a: MpAsset { address: 0xb2f82d0f38dc453d596ad40a37799446cc89274a, quantity: 3769742731863919824, price: 79076094821439265491948075126, target_share: 10 }}, context: MpContext { sharePrice: 79228162514264337593543950336, oldTotalSupply: 16967927670245107494863087, totalSupplyDelta: 0, totalTargetShares: 50, deviationIncreaseFee: 0, deviationLimit: 4294967296, feeToCashbackRatio: 0, baseFee: 0, managementBaseFee: 0, deviationFees: 0, collectedCashbacks: 0, collectedFees: 0, managementFeeRecepient: 0x65fc395ec32d69551b3966f8e5323fd233a8c9ec, oracleAddress: 0x97cd13624bb12d4ec39469b140f529459d5d369d }, cap: 138716636537490445455832 }