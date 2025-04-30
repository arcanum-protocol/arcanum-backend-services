use crate::{error::AppError, routes::stringify};
use alloy::{primitives::Address, providers::Provider};
use axum::extract::{Multipart, Query, State};
use axum_msgpack::MsgPack;
use bigdecimal::BigDecimal;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use anyhow::anyhow;
use std::{fs::create_dir_all, sync::Arc};
use tokio::{fs::File, io::AsyncWriteExt};

pub async fn list<P: Provider>(State(state): State<Arc<crate::AppState<P>>>) -> MsgPack<Value> {
    serde_json::to_value(
        state
            .stats_cache
            .iter()
            .map(|r| {
                json!({
                "address": r.key(), 
                "stats": r.value().stats,})
            })
            .collect::<Vec<Value>>(),
    )
    .unwrap()
    .into()
}

pub async fn create<P: Provider>(
    State(state): State<Arc<crate::AppState<P>>>,
    mut multipart: Multipart,
) -> Result<MsgPack<Value>, String> {
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
        .execute(&mut *state.connection.acquire().await.unwrap())
        .await.map_err(stringify)
        .map(|_| json!(()).into())
}

#[derive(Deserialize)]
pub struct PositionsRequest {
    account: Address,
}

pub async fn positions<P: Provider>(
    Query(query): Query<PositionsRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> MsgPack<Vec<DbPositions>> {
    sqlx::query_as("SELECT * FROM positions WHERE chain_id = $1 and account = $2")
        .bind::<i64>(state.chain_id as i64)
        .bind::<[u8; 20]>(query.account.into())
        .fetch_all(&mut *state.connection.acquire().await.unwrap())
        .await
        .unwrap()
        .into()
}

#[derive(Serialize, sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct DbPositions {
    #[serde(serialize_with = "serialize_address")]
    multipool: [u8; 20],
    quantity: BigDecimal,
    profit: BigDecimal,
    los: BigDecimal,
    opened_at: i64,
}

#[derive(Deserialize)]
pub struct PositionsHistoryRequest {
    account: Address,
}

pub async fn positions_history<P: Provider>(
    Query(query): Query<PositionsHistoryRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> MsgPack<Vec<DbPositions>> {
    sqlx::query_as("SELECT * FROM positions_history WHERE chain_id = $1 and account = $2")
        .bind::<i64>(state.chain_id as i64)
        .bind::<[u8; 20]>(query.account.into())
        .fetch_all(&mut *state.connection.acquire().await.unwrap())
        .await
        .unwrap()
        .into()
}

#[derive(Serialize, sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct DbPositionsHistory {
    #[serde(serialize_with = "serialize_address")]
    multipool: [u8; 20],
    pnl_quantity: BigDecimal,
    pnl_percent: BigDecimal,
    opened_at: i64,
    closed_at: i64,
}

pub fn serialize_address<S>(bytes: &[u8; 20], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Convert byte array to Address
    let address = Address::from(*bytes);
    // Serialize as a hex string with 0x prefix
    serializer.serialize_str(&address.to_string())
}
