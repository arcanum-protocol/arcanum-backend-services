use serde_json::{json, Value};

use alloy::primitives::Address;
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
    chain_id: i64,
}

pub async fn history(
    Query(query): Query<HistoryRequest>,
    State(client): State<Arc<sqlx::PgPool>>,
) -> Json<Value> {
    let to = &query.to;
    let countback = query.countback;
    let resolution: i32 = if query.resolution == "1D" {
        1440 * 60
    } else {
        let parsed_number: Result<i32, _> = query.resolution.parse();
        match parsed_number {
            Ok(num) => num * 60,
            Err(err) => return json!({"err":err.to_string()}).into(),
        }
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
            AND chain_id = $3
        ORDER BY 
            ts DESC
        LIMIT $4;",
    )
    .bind(to)
    .bind(resolution)
    .bind::<&[u8]>(query.multipool_address.as_slice())
    .bind(countback)
    .bind(query.chain_id)
    .fetch_all(client.as_ref())
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
    chain_id: i64,
    multipool_address: Address,
}

pub async fn stats(
    Query(query): Query<StatsRequest>,
    State(client): State<Arc<sqlx::PgPool>>,
) -> Json<Value> {
    let result = sqlx::query(
        "
                SELECT
                    multipool,
                    change_24h::TEXT,
                    low_24h::TEXT,
                    high_24h::TEXT,
                    current_price::TEXT
                FROM multipools
                WHERE 
                    multipool = $1
                    AND chain_id = $2;
            ",
    )
    .bind::<&[u8]>(query.multipool_address.as_slice())
    .bind(query.chain_id)
    .fetch_all(client.as_ref())
    .await;

    match result {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                let mp_id: String = row.get("multipool");
                let change_24h: String = row.get("change_24h");
                let low_24h: String = row.get("low_24h");
                let high_24h: String = row.get("high_24h");
                let current_price: String = row.get("current_price");
                json!({"multipool_id":mp_id,"change_24h":change_24h,"low_24h":low_24h,"high_24h":high_24h,"current_price":current_price}).into()
            } else {
                json!({"err":"no_data"}).into()
            }
        }
        Err(err) => json!({"err":err.to_string()}).into(),
    }
}
