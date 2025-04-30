use crate::service::log_target::GatewayTarget::PriceFetcher;
use crate::service::metrics::{DATABASE_REQUEST_DURATION_MS, PRICE_FETCHER_HEIGHT};
use crate::service::termination_codes::PRICE_FETCH_FAILED;
use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, MULTICALL3_ADDRESS};
use backend_service::logging::LogTarget;
use backend_service::KeyValue;
use bigdecimal::{BigDecimal, Num};
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
    //TODO: maybe different block fetching
    let mut latest_block = provider.get_block_number().await?;

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
                // Fetch by block number
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
            //TODO: maybe different block fetching
            latest_block = provider.get_block_number().await?;
        }

        tokio::time::sleep(Duration::from_millis(config.retry_delay_ms)).await;
    }
}

pub async fn get_mps_prices<P: Provider>(
    mps: &[Address],
    provider: &P,
    block_number: u64,
) -> anyhow::Result<(Vec<Option<U256>>, u64)> {
    let multipool_functions = multipool_types::Multipool::abi::functions();
    let get_price_func = &multipool_functions.get("getSharePricePart").unwrap()[0];
    let mut mc = alloy_multicall::Multicall::new(
        &provider,
        MULTICALL3_ADDRESS,
        //alloy::primitives::address!("cA11bde05977b3631167028862bE2a173976CA11"),
    );
    for mp in mps.iter() {
        mc.add_call(
            *mp,
            &get_price_func,
            &[
                DynSolValue::Uint(U256::MAX, 256),
                DynSolValue::Uint(U256::ZERO, 256),
            ],
            true,
        )
    }

    let overflow_error = alloy::primitives::bytes!(
        "0x4e487b710000000000000000000000000000000000000000000000000000000000000012"
    );

    mc.add_get_current_block_timestamp();
    let mut res = match mc.call_with_block(block_number.into()).await {
        Ok(res) => res,
        Err(e) => PriceFetcher
            .error(json!({
                "m": "multipool price fetch failed",
                "b": block_number,
                "e": e.to_string()
            }))
            .terminate(PRICE_FETCH_FAILED),
    };
    let ts = res.pop().unwrap().unwrap().as_uint().unwrap().0;
    let prices: Vec<Option<U256>> = res
        .into_iter()
        .map(|p| match p {
            Ok(p) => Some(p.as_uint().unwrap().0),
            Err(e) if e == overflow_error => None,
            //TODO: warning here
            Err(_) => None,
        })
        .collect();

    Ok((prices, ts.to()))
}
