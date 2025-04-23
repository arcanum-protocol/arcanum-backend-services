use serde_json::{json, Value};

use crate::routes::stringify;
use alloy::{primitives::Address, providers::Provider};
use axum::{
    extract::{Multipart, Query, State},
    Json,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Row};

use anyhow::anyhow;
use std::sync::Arc;
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Deserialize)]
pub struct PnlRequest {
    account: Address,
    chain_id: i64,
}

#[derive(Serialize)]
pub struct PnlResponse {
    account: Address,
    chain_id: i64,
    acc_profit: BigDecimal,
    acc_loss: BigDecimal,
    open_quote: BigDecimal,
    close_quote: BigDecimal,
    timestamp: i64,
}

impl sqlx::FromRow<'_, PgRow> for PnlResponse {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        let account: Vec<u8> = row.try_get("account")?;
        Ok(Self {
            account: Address::from_slice(account.as_slice()),
            chain_id: row.try_get("chain_id")?,
            acc_profit: row.try_get("acc_profit")?,
            acc_loss: row.try_get("acc_loss")?,
            open_quote: row.try_get("open_quote")?,
            close_quote: row.try_get("close_quote")?,
            timestamp: row.try_get("timestamp")?,
        })
    }
}

pub async fn pnl<P: Provider>(
    Query(query): Query<PnlRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> Result<Json<PnlResponse>, String> {
    sqlx::query_as(
        "select * from pnl where account = $1 and chain_id = $2 order by timestamp desc limit 1",
    )
    .bind(query.account.as_slice())
    .bind(query.chain_id)
    .fetch_one(&state.connection)
    .await
    .map(Json)
    .map_err(stringify)
}

#[derive(Deserialize)]
pub struct PositionRequest {
    multipool: Option<Address>,
    account: Address,
    chain_id: i64,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct PositionResponse {
    quantity: BigDecimal,
    opened_at: i64,
    acc_profit: BigDecimal,
    acc_loss: BigDecimal,
    open_quantity: BigDecimal,
    open_price: BigDecimal,
    close_quantity: BigDecimal,
    close_price: BigDecimal,
    timestamp: i64,
}

pub async fn positions<P: Provider>(
    Query(query): Query<PositionRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> Result<Json<Vec<PositionResponse>>, String> {
    let query = if let Some(mp) = query.multipool {
        sqlx::query_as(
            "select * from positions_pnl 
                left join positions 
                on positions.account = positions_pnl.account 
                    and positions.multipool = positions_pnl.multipool
                    and positions.chain_id = positions_pnl.chain_id 
                where positions_pnl.multipool = $1 
                    and positions_pnl.account = $2 
                    and positions_pnl.chain_id = $3 
                order by timestamp desc limit 1",
        )
        .bind(mp.to_vec())
        .bind(query.account.as_slice())
        .bind(query.chain_id)
    } else {
        sqlx::query_as(
            "select * from positions_pnl 
                left join positions 
                on positions.account = positions_pnl.account 
                and positions.chain_id = positions_pnl.chain_id where positions_pnl.account = $1 and positions_pnl.chain_id = $2",
        )
        .bind(query.account.as_slice())
        .bind(query.chain_id)
    };

    query
        .fetch_all(&state.connection)
        .await
        .map(|v| Json(v))
        .map_err(stringify)
}

#[derive(Deserialize)]
pub struct HistoryRequest {
    asset_address: Address,
    chain_id: Option<i64>,
}

#[derive(Serialize)]
pub struct HistoryResponse {
    asset: Address,
}

impl sqlx::FromRow<'_, PgRow> for HistoryResponse {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        let address: Vec<u8> = row.try_get("asset")?;
        Ok(Self {
            asset: Address::from_slice(address.as_slice()),
        })
    }
}

pub async fn history<P: Provider>(
    Query(query): Query<HistoryRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> Result<Json<PositionResponse>, String> {
    let query = if let Some(chain_id) = query.chain_id {
        sqlx::query_as("select * from assets where asset = $1 and chain_id = $2")
            .bind(query.asset_address.as_slice())
            .bind(chain_id)
    } else {
        sqlx::query_as("select * from assets where asset = $1").bind(query.asset_address.as_slice())
    };
    query
        .fetch_one(&state.connection)
        .await
        .map(|v| Json(v))
        .map_err(stringify)
}
