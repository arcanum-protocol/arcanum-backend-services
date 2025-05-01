use axum_msgpack::MsgPack;
use serde_json::Value;

use alloy::{primitives::Address, providers::Provider};
use axum::extract::{Query, State};
use serde::Deserialize;
use std::sync::Arc;

use crate::cache::{try_resolution_to_index, Candle, DbCandleSmall, Stats};

#[derive(Deserialize)]
pub struct HistoryRequest {
    t: Option<u64>,
    c: Option<usize>,
    r: i32,
    m: Address,
}

pub async fn candles<P: Provider>(
    Query(query): Query<HistoryRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> MsgPack<Vec<Candle>> {
    //TODO: error handling
    let resolution_index = try_resolution_to_index(query.r).unwrap();

    let candles =
        state.stats_cache.get(&query.m).unwrap().value().candles[resolution_index].clone();

    let (ts, countback) = match (query.t, query.c) {
        (Some(ts), Some(countback)) => (ts, countback),
        _ => return candles.into(),
    };

    let result: Vec<DbCandleSmall> = sqlx::query_as(
        "
          SELECT
              open,
              close,
              low,
              hight,
              ts
          FROM
              candles
          WHERE
              ts <= $1
              AND resolution = $2
              AND multipool = $3
          ORDER BY
              ts DESC
          LIMIT $4;",
    )
    .bind(ts as i64)
    .bind(query.r)
    .bind::<&[u8]>(query.m.as_slice())
    .bind(countback as i64)
    .fetch_all(&mut *state.connection.acquire().await.unwrap())
    .await
    .unwrap();
    result
        .into_iter()
        .map(Into::into)
        .rev()
        .collect::<Vec<Candle>>()
        .into()
}

#[derive(Deserialize)]
pub struct StatsRequest {
    m: Address,
}

pub async fn stats<P: Provider>(
    Query(query): Query<StatsRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> MsgPack<Stats> {
    state
        .stats_cache
        .get(&query.m)
        .unwrap()
        .value()
        .stats
        .clone()
        .into()
}
