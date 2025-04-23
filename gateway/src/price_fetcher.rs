use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, MULTICALL3_ADDRESS};
use bigdecimal::{BigDecimal, Num};
use serde::Deserialize;
use sqlx::Acquire;
use sqlx::{postgres::PgRow, Row};
use std::ops::Shl;
use std::sync::Arc;
use std::time::Duration;

use crate::cache::AppState;

#[derive(Deserialize)]
pub struct PriceFetcherConfig {
    pub block_delay: u64,
    pub multipools_in_chunk: u64,
    pub retry_delay_ms: u64,
}

//TODO: commit checed blocks into db somehow
//TODO: make all atomic txn
pub async fn run<P: Provider>(
    app_state: Arc<AppState<P>>,
    config: PriceFetcherConfig,
) -> anyhow::Result<()> {
    let pool = &app_state.connection;
    let provider = &app_state.provider;
    let chain_id = provider.get_chain_id().await? as i64;

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
                for (price, mp) in prices.into_iter().zip(chunk) {
                    sqlx::query("call insert_price($1, $2, $3, $4)")
                        .bind(chain_id)
                        .bind::<&[u8]>(mp.as_slice())
                        .bind::<i64>(ts as i64)
                        .bind::<BigDecimal>(BigDecimal::from_str_radix(
                            price.to_string().as_str(),
                            10,
                        )?)
                        .execute(&mut *transaction)
                        .await?;

                    app_state
                        .stats_cache
                        .get_mut(mp)
                        .unwrap()
                        .insert_price(price, ts);
                }
            }

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
) -> anyhow::Result<(Vec<u128>, u64)> {
    let multipool_functions = multipool_types::Multipool::abi::functions();
    let get_price_func = &multipool_functions.get("getSharePricePart").unwrap()[0];
    let mut mc = alloy_multicall::Multicall::new(
        &provider,
        MULTICALL3_ADDRESS, // address!("cA11bde05977b3631167028862bE2a173976CA11"),
    );
    for mp in mps.iter() {
        mc.add_call(
            *mp,
            get_price_func,
            &[
                DynSolValue::Uint(U256::MAX, 256),
                DynSolValue::Uint(U256::ZERO, 256),
            ],
            true,
        );
    }

    mc.add_get_current_block_timestamp();
    let mut res = mc.call_with_block(block_number.into()).await?;
    let ts = res.pop().unwrap().unwrap().as_uint().unwrap().0;
    let prices: Vec<u128> = res
        .into_iter()
        .map(|p| p.unwrap().as_uint().unwrap().0.to())
        .collect();

    Ok((prices, ts.to()))
}
