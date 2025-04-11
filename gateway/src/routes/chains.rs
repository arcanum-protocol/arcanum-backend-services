use serde_json::{json, Value};

use crate::routes::stringify;
use alloy::primitives::Address;
use axum::{
    extract::{Multipart, Query, State},
    Json,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Row};

use anyhow::anyhow;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ChainsRequest {
    asset_address: Address,
    chain_id: Option<i64>,
}

#[derive(Serialize)]
pub struct ChainsResponse {
    chain_id: i64,
    chain_name: String,
    native_token_name: String,
    default_rpc_url: String,
    factory: Address,
    router: Address,
    logo_url: String,
}

impl sqlx::FromRow<'_, PgRow> for ChainsResponse {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        let address: Vec<u8> = row.try_get("asset")?;
        Ok(Self {
            asset: Address::from_slice(address.as_slice()),
            chain_id: row.try_get("chain_id")?,
            is_primary: row.try_get("is_primary")?,
            name: row.try_get("name")?,
            symbol: row.try_get("symbol")?,
            logo_url: row.try_get("logo_url")?,
            description: row.try_get("description")?,
        })
    }
}

pub async fn chains(
    Query(query): Query<ChainsRequest>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<ChainsResponse>, String> {
    let query = if let Some(chain_id) = query.chain_id {
        sqlx::query_as("select * from assets where asset = $1 and chain_id = $2")
            .bind(query.asset_address.as_slice())
            .bind(chain_id)
    } else {
        sqlx::query_as("select * from assets where asset = $1").bind(query.asset_address.as_slice())
    };
    query
        .fetch_one(&state.pool)
        .await
        .map(|v| Json(v))
        .map_err(stringify)
}
