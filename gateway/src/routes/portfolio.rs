use crate::{
    error::{AppError, AppResult},
    service::metrics::DATABASE_REQUEST_DURATION_MS,
};
use alloy::{
    primitives::{Address, B256},
    providers::Provider,
    rpc::types::Filter,
    sol_types::{SolEvent, SolEventInterface},
};
use arweave_client::{Tag, Transaction, Uploader};
//use arweave_client::{Rpc, Tag, Transaction, Uploader};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_msgpack::MsgPack;
use backend_service::KeyValue;
use bigdecimal::BigDecimal;
use multipool_types::MultipoolFactory::{self, MultipoolFactoryEvents};
use serde::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use std::{sync::Arc, time::Instant};

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

#[derive(Deserialize)]
pub struct CreateRequest {
    #[serde(with = "base64")]
    #[serde(rename = "l")]
    logo_bytes: Vec<u8>,
    #[serde(rename = "st")]
    salt: B256,
    #[serde(rename = "ih")]
    init_code_hash: B256,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "n")]
    name: String,
    #[serde(rename = "d")]
    description: String,
}

#[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct DbCreateMultipools {
    description: Option<String>,
}

pub async fn create<P: Provider>(
    State(state): State<Arc<crate::AppState<P>>>,
    MsgPack(form): MsgPack<CreateRequest>,
) -> AppResult<MsgPack<()>> {
    let multipool = state.factory.create2(form.salt, form.init_code_hash);

    let current_block = state.provider.get_block_number().await?;

    //TODO: call only if not found in DB of cache
    //maybe make other function of answer y/n multipool real, if arwave exist do we need to put sth
    //there
    let filter = Filter::new()
        .event(MultipoolFactory::MultipoolCreated::SIGNATURE)
        .topic1(multipool)
        .from_block(current_block - state.log_search_interval)
        .address(state.factory);
    let events = state.provider.get_logs(&filter).await?;

    let (fee_receiver, fee_amount) = events
        .into_iter()
        .find_map(
            |i| match MultipoolFactoryEvents::decode_log(&i.into()).ok()?.data {
                MultipoolFactoryEvents::MultipoolCreated(e) => Some((e.feeReceiver, e.feeAmount)),
                _ => None,
            },
        )
        .ok_or(AppError::MultipoolNotCreated)?;

    if form.description.len() > 500
        && form.name.len() > 25
        && form.symbol.len() > 10
        && form.logo_bytes.len() > 1024 * 100
    {
        Err(AppError::InvalidPayloadSize)?;
    }

    let mut conn = state
        .connection
        .acquire()
        .await
        .map_err(|_| AppError::DbIsBusy)?;

    let multipool_record: Option<DbCreateMultipools> =
        sqlx::query_as("SELECT description FROM multipools WHERE multipool = $1")
            .bind::<[u8; 20]>(multipool.into())
            .fetch_optional(&mut *conn)
            .await?;

    if multipool_record.and_then(|r| r.description).is_some() {
        Err(AppError::MetadataAlreadySet)?;
    }

    let timer = Instant::now();
    sqlx::query(
        "INSERT INTO
            multipools(multipool, chain_id, owner, name, symbol, description, logo)
        VALUES
            ($1,$2,$3,$4,$5,$6,$7)
        ON CONFLICT
            (multipool)
        DO UPDATE SET
            logo = $7,
            description = $6;
        ",
    )
    .bind::<[u8; 20]>(multipool.into())
    .bind(state.chain_id as i64)
    .bind::<[u8; 20]>(Address::ZERO.into())
    .bind(&form.name)
    .bind(&form.symbol)
    .bind(&form.description)
    .bind(&form.logo_bytes)
    .fetch_all(&mut *conn)
    .await?;
    DATABASE_REQUEST_DURATION_MS.record(
        timer.elapsed().as_millis() as u64,
        &[KeyValue::new("query_name", "mp_dnl")],
    );

    let arweave = match &state.arweave {
        Some(a) => {
            if a.treasury_addresses.contains(&fee_receiver) && fee_amount >= a.fee_amount {
                a
            } else {
                return Ok(().into());
            }
        }
        None => return Ok(().into()),
    };

    let name_bytes = form.name.into_bytes();
    let desc_offset = name_bytes.len();
    let desc_bytes = form.description.into_bytes();
    let logo_offset = desc_offset + desc_bytes.len();
    let data = name_bytes
        .into_iter()
        .chain(desc_bytes)
        .chain(form.logo_bytes)
        .collect();

    let mut tx = Transaction::builder(arweave.rpc.clone())
        .tags(vec![
            Tag {
                name: "Content-Type".to_string(),
                value: "MpData".to_string(),
            },
            Tag {
                name: "Address".to_string(),
                value: multipool.to_string(),
            },
            Tag {
                name: "ChainId".to_string(),
                value: state.chain_id.to_string(),
            },
            Tag {
                name: "Symbol".to_string(),
                value: form.symbol.to_owned(),
            },
            Tag {
                name: "DescriptionOffset".to_string(),
                value: desc_offset.to_string(),
            },
            Tag {
                name: "LogoOffset".to_string(),
                value: logo_offset.to_string(),
            },
        ])
        .data(data)
        .build()
        .await?;
    tx.sign(arweave.signer.clone())?;
    let mut uploader = Uploader::new(arweave.rpc.clone(), tx);
    uploader.upload_chunks().await?;

    return Ok(().into());
}

#[derive(Deserialize)]
pub struct MetadataRequest {
    #[serde(rename = "m")]
    multipools: String,
}

#[derive(Serialize, sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct DbMetadata {
    #[serde(serialize_with = "serialize_address")]
    #[serde(rename = "m")]
    multipool: [u8; 20],
    #[serde(with = "base64")]
    #[serde(rename = "l")]
    logo: Vec<u8>,
    #[serde(rename = "d")]
    description: String,
}

pub async fn metadata<P: Provider>(
    Query(query): Query<MetadataRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> AppResult<MsgPack<Vec<DbMetadata>>> {
    let multipools = serde_json::from_str::<Vec<Address>>(&query.multipools)
        .map_err(|_| AppError::FailedToParse)?;
    sqlx::query_as("SELECT multipool, logo, description FROM multipools WHERE logo IS NOT NULL and description IS NOT NULL and multipool in (SELECT unnest($1::address[]))")
        .bind::<Vec<[u8; 20]>>(multipools.into_iter().map(Into::into).collect())
        .fetch_all(&mut *state.connection.acquire().await.unwrap())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[derive(Deserialize)]
pub struct PositionsRequest {
    #[serde(rename = "a")]
    account: Address,
}

pub async fn positions<P: Provider>(
    Query(query): Query<PositionsRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> AppResult<MsgPack<Vec<DbPositions>>> {
    sqlx::query_as("SELECT * FROM positions WHERE chain_id = $1 and account = $2")
        .bind::<i64>(state.chain_id as i64)
        .bind::<[u8; 20]>(query.account.into())
        .fetch_all(&mut *state.connection.acquire().await.unwrap())
        .await
        .map(Into::into)
        .map_err(Into::into)
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
    loss: BigDecimal,
    #[serde(rename(serialize = "o"))]
    opened_at: i64,
}

#[derive(Deserialize)]
pub struct PositionsHistoryRequest {
    #[serde(rename = "a")]
    account: Address,
}

pub async fn positions_history<P: Provider>(
    Query(query): Query<PositionsHistoryRequest>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> AppResult<MsgPack<Vec<DbPositionsHistory>>> {
    sqlx::query_as("SELECT * FROM positions_history WHERE chain_id = $1 and account = $2")
        .bind::<i64>(state.chain_id as i64)
        .bind::<[u8; 20]>(query.account.into())
        .fetch_all(&mut *state.connection.acquire().await.unwrap())
        .await
        .map(Into::into)
        .map_err(Into::into)
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

mod base64 {
    use base64::prelude::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(key: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&BASE64_STANDARD.encode(key))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer).and_then(|string| {
            BASE64_STANDARD
                .decode(&string)
                .map_err(|err| Error::custom(err.to_string()))
        })
    }
}
