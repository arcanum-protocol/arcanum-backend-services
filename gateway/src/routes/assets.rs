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
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Deserialize)]
pub struct AssetRequest {
    asset_address: Address,
    chain_id: Option<i64>,
}

#[derive(Serialize)]
pub struct AssetResponse {
    asset: Address,
    chain_id: i64,
    is_primary: bool,
    name: String,
    symbol: String,
    logo_url: String,
    description: String,
}

impl sqlx::FromRow<'_, PgRow> for AssetResponse {
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

pub async fn asset(
    Query(query): Query<AssetRequest>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<AssetResponse>, String> {
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

#[derive(Deserialize)]
pub struct AssetsListRequest {
    multipool_address: Option<Address>,
    chain_id: Option<i64>,
}

pub async fn list(
    Query(query): Query<AssetsListRequest>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Vec<AssetResponse>>, String> {
    let query = if let Some(mp_address) = query.multipool_address {
        sqlx::query_as("select * from assets where asset = $1").bind(mp_address.to_vec())
    } else if let Some(chain_id) = query.chain_id {
        sqlx::query_as("select * from assets where chain_id = $1").bind(chain_id)
    } else {
        sqlx::query_as("select * from assets")
    };
    query
        .fetch_all(&state.pool)
        .await
        .map(|v| Json(v))
        .map_err(stringify)
}
