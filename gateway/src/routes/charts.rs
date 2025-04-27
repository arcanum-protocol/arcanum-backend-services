use serde_json::{json, Value};

use alloy::{primitives::Address, providers::Provider};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use sqlx::{postgres::PgRow, Row};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct HistoryRequest {
    to: i64,
    countback: i64,
    resolution: String,
    multipool_address: Address,
}

pub async fn candles<P: Provider>(
    Query(query): Query<HistoryRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> Json<Value> {
    let to = &query.to;
    let countback = query.countback;

    let parsed_number: Result<i32, _> = query.resolution.parse();

    let resolution: i32 = match parsed_number {
        Ok(num) => num * 60,
        Err(err) => return json!({"err":err.to_string()}).into(),
    };
    let result = sqlx::query(
        "
        SELECT 
            open::TEXT as o, 
            close::TEXT as c, 
            low::TEXT as l, 
            high::TEXT as h, 
            ts::TEXT as t
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
    .bind(to)
    .bind(resolution)
    .bind::<&[u8]>(query.multipool_address.as_slice())
    .bind(countback)
    .fetch_all(&mut *state.connection.acquire().await.unwrap())
    .await;

    match result {
        Ok(rows) => {
            if rows.is_empty() {
                json!({"s": "no_data"}).into()
            } else {
                json!({
                    "s":"ok",
                    "t": rows.iter().rev().map(|r: &PgRow| r.get("t")).collect::<Vec<String>>(),
                    "o": rows.iter().rev().map(|r: &PgRow| r.get("o") ).collect::<Vec<String>>(),
                    "c": rows.iter().rev().map(|r: &PgRow| r.get("c") ).collect::<Vec<String>>(),
                    "l": rows.iter().rev().map(|r: &PgRow| r.get("l") ).collect::<Vec<String>>(),
                    "h": rows.iter().rev().map(|r: &PgRow| r.get("h") ).collect::<Vec<String>>(),
                })
                .into()
            }
        }
        Err(err) => {
            println!("{:?}", err);
            json!({"s":"error"}).into()
        }
    }
}

#[derive(Deserialize)]
pub struct StatsRequest {
    multipool_address: Address,
}

pub async fn stats<P: Provider>(
    Query(query): Query<StatsRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> Json<Value> {
    serde_json::to_value(
        state
            .stats_cache
            .get(&query.multipool_address)
            .unwrap()
            .value()
            .stats
            .clone(),
    )
    .unwrap()
    .into()
}
