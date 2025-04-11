use crate::{error::AppError, routes::stringify};
use alloy::primitives::Address;
use axum::{
    extract::{Multipart, Query, State},
    Json,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgRow, Row};

use anyhow::anyhow;
use std::{fs::create_dir_all, sync::Arc};
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Deserialize)]
pub struct PortfolioListRequest {
    chain_id: Option<i64>,
}

#[derive(Serialize)]
pub struct MultipoolResponse {
    name: Option<String>,
    symbol: Option<String>,
    description: Option<String>,
    chain_id: i64,
    multipool: Address,
    change_24h: BigDecimal,
    low_24h: BigDecimal,
    high_24h: BigDecimal,
    current_price: BigDecimal,
    total_supply: BigDecimal,
}

impl sqlx::FromRow<'_, PgRow> for MultipoolResponse {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        let address: Vec<u8> = row.try_get("multipool")?;
        Ok(Self {
            name: row.try_get("name")?,
            symbol: row.try_get("symbol")?,
            description: row.try_get("description")?,
            chain_id: row.try_get("chain_id")?,
            multipool: Address::from_slice(address.as_slice()),
            change_24h: row.try_get("change_24h")?,
            low_24h: row.try_get("low_24h")?,
            high_24h: row.try_get("high_24h")?,
            current_price: row.try_get("current_price")?,
            total_supply: row.try_get("total_supply")?,
        })
    }
}

// TODO: portfolio list
pub async fn list(
    Query(query): Query<PortfolioListRequest>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Vec<MultipoolResponse>>, String> {
    if let Some(id) = query.chain_id {
        sqlx::query_as("select * from multipools where chain_id = $1")
            .bind(id)
            .fetch_all(&state.pool)
            .await
            .map(|v| Json(v))
            .map_err(stringify)
    } else {
        sqlx::query_as("select * from multipools")
            .fetch_all(&state.pool)
            .await
            .map(|v| Json(v))
            .map_err(stringify)
    }
}

pub async fn create(
    State(state): State<Arc<crate::AppState>>,
    mut multipart: Multipart,
) -> Result<Json<Value>, String> {
    let mut logo = None;
    let mut multipool_address: Option<Address> = None;
    let mut chain_id: Option<i64> = None;
    let mut symbol: Option<_> = None;
    let mut name: Option<_> = None;
    let mut description: Option<_> = None;

    while let Some(mut field) = multipart.next_field().await.map_err(stringify)? {
        let field_name = field.name().unwrap_or_default().to_string();

        if field_name == "logo" {
            let mut file_data = Vec::new();
            while let Some(chunk) = field.chunk().await.map_err(stringify)? {
                file_data.extend_from_slice(&chunk);
            }
            logo = Some(file_data);
        } else if field_name == "name" {
            name = Some(field.text().await.map_err(stringify)?);
        } else if field_name == "chain_id" {
            chain_id = Some(
                field
                    .text()
                    .await
                    .map_err(stringify)?
                    .parse()
                    .map_err(stringify)?,
            );
        } else if field_name == "symbol" {
            symbol = Some(field.text().await.map_err(stringify)?);
        } else if field_name == "description" {
            description = Some(field.text().await.map_err(stringify)?);
        } else if field_name == "multipool_address" {
            multipool_address = Some(
                field
                    .text()
                    .await
                    .map_err(stringify)?
                    .parse()
                    .map_err(stringify)?,
            );
        }
    }

    let logo = logo.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let multipool_address = multipool_address
        .ok_or(anyhow!("Invalid form"))
        .map_err(stringify)?;
    let chain_id = chain_id.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let symbol = symbol.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let name = name.ok_or(anyhow!("Invalid form")).map_err(stringify)?;
    let description = description
        .ok_or(anyhow!("Invalid form"))
        .map_err(stringify)?;
    //TODO: add limits on name, symbol, description + logo size

    let file_path = format!("/logos/{multipool_address}");
    let _ = create_dir_all(&file_path);
    let mut file_handle = File::create(file_path).await.map_err(stringify)?;
    file_handle.write_all(&logo).await.map_err(stringify)?;

    sqlx::query("INSERT INTO multipools(chain_id, multipool, name, symbol, description) VALUES($1,$2,$3,$4,$5);")
        .bind(chain_id)
        .bind::<&[u8]>(multipool_address.as_slice())
        .bind(name)
        .bind(symbol)
        .bind(description)
        .execute(&mut *state.pool.acquire().await.unwrap())
        .await.map_err(stringify)
        .map(|_| json!(()).into())
}

#[derive(Deserialize)]
pub struct PortfolioRequest {
    chain_id: i64,
    multipool_address: Address,
}

// TODO portfolio info from storage
pub async fn portfolio(
    Query(query): Query<PortfolioRequest>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Value>, AppError> {
    let multipool_data = state
        .getters
        .getters
        .get(&query.multipool_address)
        .map(|f| f());

    let data = sqlx::query(
        "
    select 
        name, symbol, description, change_24h, low_24h, high_24h, current_price, total_supply 
    from 
        multipools 
    where 
        multipool = $1 and chain_id = $2
        ",
    )
    .bind(query.multipool_address.as_slice())
    .bind(query.chain_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(json!({
        "cache": multipool_data,
        "name": data.try_get::<String, &str>("name").ok(),
        "symbol": data.try_get::<String, &str>("symbol").ok(),
        "description": data.try_get::<String, &str>("description").ok(),
        "change_24h": data.try_get::<BigDecimal, &str>("change_24h").ok(),
        "low_24h": data.try_get::<BigDecimal, &str>("low_24h").ok(),
        "high_24h": data.try_get::<BigDecimal, &str>("high_24h").ok(),
        "current_price": data.try_get::<BigDecimal, &str>("current_price").ok(),
        "total_supply": data.try_get::<BigDecimal, &str>("total_supply").ok(),
    })
    .into())
}
