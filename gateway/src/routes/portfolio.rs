use crate::{error::AppError, routes::stringify};
use alloy::{primitives::Address, providers::Provider};
use arweave_client::{Rpc, Tag, Transaction, Uploader};
use axum::{
    extract::{Multipart, Path, Query, State},
    Json,
};
use axum_msgpack::MsgPack;
use bigdecimal::BigDecimal;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use anyhow::anyhow;
use std::sync::Arc;

pub async fn list<P: Provider>(State(state): State<Arc<crate::AppState<P>>>) -> MsgPack<Value> {
    serde_json::to_value(
        state
            .stats_cache
            .iter()
            .map(|r| {
                json!({
                "a": r.key(), 
                "s": r.value().stats,})
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
    let name_bytes = name.into_bytes();
    let desc_offset = name_bytes.len();
    let desc_bytes = description.into_bytes();
    let logo_offset = desc_offset + desc_bytes.len();
    // let data = name_bytes
    //     .into_iter()
    //     .chain(desc_bytes)
    //     .chain(logo)
    //     .collect();

    // let mut tx = Transaction::builder(state.arweave_rpc.clone())
    //     .tags(vec![
    //         Tag {
    //             name: "Content-Type".to_string(),
    //             value: "MpData".to_string(),
    //         },
    //         Tag {
    //             name: "Address".to_string(),
    //             value: multipool_address.to_string(),
    //         },
    //         Tag {
    //             name: "ChainId".to_string(),
    //             value: chain_id.to_string(),
    //         },
    //         Tag {
    //             name: "Symbol".to_string(),
    //             value: symbol.to_owned(),
    //         },
    //         Tag {
    //             name: "DescriptionOffset".to_string(),
    //             value: desc_offset.to_string(),
    //         },
    //         Tag {
    //             name: "LogoOffset".to_string(),
    //             value: logo_offset.to_string(),
    //         },
    //     ])
    //     .data(data)
    //     .build()
    //     .await
    //     .map_err(stringify)?;
    // tx.sign(state.arweave_signer.clone()).map_err(stringify)?;
    // let mut uploader = Uploader::new(state.arweave_rpc.clone(), tx);
    // uploader.upload_chunks().await.unwrap();

    //TODO: how to not fkn ddos
    // sqlx::query("INSERT INTO multipools(logo, description) ")
    //     .bind::<[u8; 20]>(multipool.into())
    //     .fetch_all(&mut *state.connection.acquire().await.unwrap())
    //     .await
    //     .unwrap()
    //     .into()
    //TODO: add limits on name, symbol, description + logo size
    Ok(json!(()).into())
}

#[derive(Deserialize)]
pub struct MetadataRequest {
    multipool: Address,
}

#[derive(Serialize, sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct DbMetadata {
    #[serde(with = "serde_bytes")]
    #[serde(rename(serialize = "l"))]
    logo: Vec<u8>,
    #[serde(rename(serialize = "d"))]
    description: String,
}

pub async fn metadata<P: Provider>(
    Path(multipool): Path<Address>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> MsgPack<Vec<DbPositions>> {
    sqlx::query_as("SELECT logo, description FROM multipools WHERE multipool = $1")
        .bind::<[u8; 20]>(multipool.into())
        .fetch_all(&mut *state.connection.acquire().await.unwrap())
        .await
        .unwrap()
        .into()
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
    #[serde(rename(serialize = "m"))]
    multipool: [u8; 20],
    #[serde(rename(serialize = "q"))]
    quantity: BigDecimal,
    #[serde(rename(serialize = "p"))]
    profit: BigDecimal,
    #[serde(rename(serialize = "l"))]
    los: BigDecimal,
    #[serde(rename(serialize = "o"))]
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
    #[serde(rename(serialize = "q"))]
    pnl_quantity: BigDecimal,
    #[serde(rename(serialize = "p"))]
    pnl_percent: BigDecimal,
    #[serde(rename(serialize = "o"))]
    opened_at: i64,
    #[serde(rename(serialize = "c"))]
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
