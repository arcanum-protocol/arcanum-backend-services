use crate::service::log_target::GatewayTarget::PriceFetcher;
use crate::service::metrics::{DATABASE_REQUEST_DURATION_MS, PRICE_FETCHER_HEIGHT};
use crate::service::termination_codes::PRICE_FETCH_FAILED;
use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, MULTICALL3_ADDRESS};
use alloy::sol_types::SolCall;
use backend_service::logging::LogTarget;
use backend_service::KeyValue;
use bigdecimal::{BigDecimal, Num};
use multipool_types::Multicall::{self, getCurrentBlockTimestampCall};
use multipool_types::Multicall3::Call3;
use serde::Deserialize;
use serde_json::json;
use sqlx::Acquire;
use sqlx::Row;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cache::AppState;

#[derive(Deserialize)]
pub struct PriceFetcherConfig {
    pub block_delay: u64,
    pub multipools_in_chunk: u64,
    pub retry_delay_ms: u64,
}

pub async fn run<P: Provider>(
    app_state: Arc<AppState<P>>,
    config: PriceFetcherConfig,
) -> anyhow::Result<()> {
    let pool = &app_state.connection;
    let provider = &app_state.provider;
    let chain_id = provider.get_chain_id().await? as i64;

    //TODO: do something with block competition
    let mut latest_block = provider
        .get_block_by_number(BlockNumberOrTag::Finalized)
        .hashes()
        .await?
        .expect("no finalized block")
        .header
        .number;

    let mut connection = pool.acquire().await?;

    let mut indexing_block =
        sqlx::query("select block_number from price_indexes where chain_id = $1")
            .bind(chain_id)
            .fetch_optional(&mut *connection)
            .await?
            .map(|r| r.get::<i64, _>("block_number") as u64 + config.block_delay)
            .unwrap_or(latest_block);
    loop {
        if indexing_block <= latest_block {
            let mut transaction = connection.begin().await?;
            let multipools = app_state.multipools.read().unwrap().clone();

            for chunk in multipools.chunks(config.multipools_in_chunk as usize) {
                let (prices, ts) = get_mps_prices(chunk, &provider, indexing_block).await?;
                for (p, mp) in prices.into_iter().zip(chunk) {
                    if let Some(price) = p {
                        let timer = Instant::now();
                        sqlx::query("call insert_price($1,$2,$3)")
                            .bind::<[u8; 20]>(*mp.0)
                            .bind::<i64>(ts as i64)
                            .bind::<BigDecimal>(BigDecimal::from_str_radix(
                                price.to_string().as_str(),
                                10,
                            )?)
                            .execute(&mut *transaction)
                            .await?;
                        DATABASE_REQUEST_DURATION_MS.record(
                            timer.elapsed().as_millis() as u64,
                            &[KeyValue::new("query_name", "insert_price")],
                        );

                        app_state
                            .stats_cache
                            .get_mut(mp)
                            .unwrap()
                            .insert_price(price, ts);
                    }
                }
            }
            PRICE_FETCHER_HEIGHT.record(indexing_block, &[]);

            sqlx::query(
                "INSERT INTO price_indexes(chain_id, block_number) VALUES ($1, $2) ON CONFLICT (chain_id) DO UPDATE SET block_number = $2"
            )
                .bind(chain_id)
                .bind(indexing_block as i64)
                .execute(&mut *transaction).await?;

            transaction.commit().await?;

            indexing_block += config.block_delay;
        }
        if indexing_block > latest_block {
            latest_block = provider
                .get_block_by_number(BlockNumberOrTag::Finalized)
                .hashes()
                .await?
                .expect("no finalized block")
                .header
                .number;
        }

        tokio::time::sleep(Duration::from_millis(config.retry_delay_ms)).await;
    }
}

pub async fn get_mps_prices<P: Provider>(
    mps: &[Address],
    provider: &P,
    block_number: u64,
) -> anyhow::Result<(Vec<Option<U256>>, u64)> {
    let multicall = Multicall::new(MULTICALL3_ADDRESS, provider);
    let mut calls = vec![];

    for mp in mps.iter() {
        calls.push(Call3 {
            target: *mp,
            allowFailure: true,
            callData: multipool_types::Multipool::getSharePricePartCall {
                limit: U256::MAX,
                offset: U256::ZERO,
            }
            .abi_encode()
            .into(),
        })
    }

    calls.push(Call3 {
        target: MULTICALL3_ADDRESS,
        allowFailure: true,
        callData: getCurrentBlockTimestampCall {}.abi_encode().into(),
    });

    let call = multicall.aggregate3(calls).block(block_number.into());
    let calldata = call.calldata().clone();

    let mut res = match call.call().await {
        Ok(res) => res,
        Err(e) => {
            PriceFetcher
                .error(json!({
                    "m": "multipool price fetch failed",
                    "out_data_err": format!("{e:?}"),
                    "calldata": calldata,
                    "b": block_number,
                    "e": e.to_string()
                }))
                .terminate(PRICE_FETCH_FAILED);
        }
    };

    let ts = res
        .pop()
        .map(|ts| U256::try_from_be_slice(&ts.returnData).expect("Failed to decode ts"))
        .unwrap();

    let prices: Vec<Option<U256>> = res
        .into_iter()
        .map(|p| match p.success {
            true => U256::try_from_be_slice(&p.returnData),
            false => None,
        })
        .collect();

    Ok((prices, ts.to()))
}