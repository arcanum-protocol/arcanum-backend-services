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
//use arweave_client::{Rpc, Tag, Transaction, Uploader};
use axum::extract::{Query, State};
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

pub async fn create<P: Provider>(
    State(state): State<Arc<crate::AppState<P>>>,
    MsgPack(form): MsgPack<CreateRequest>,
) -> AppResult<MsgPack<()>> {
    let multipool = state.factory.create2(form.salt, form.init_code_hash);

    //TODO: maybe use indexer <-> this route method that
    //is going to be sending data to arwave
    // also need to check fees so probably transaction hash is neserarry
    // THIS ONE TO BE REMOVED AND LOGS TO BE USED
    let code = state
        .provider
        .get_code_at(multipool)
        .latest()
        .await
        .map_err(|_| AppError::FailedToGetCode)?;

    //TODO: only insert if there is no logo set before
    //TODO: same for arwave

    // let current_block = state.provider.get_block_number().await?;
    // let block_countback = 1000;

    // let filter = Filter::new()
    //     .event(MultipoolFactory::ProtocolFeeSent::SIGNATURE)
    //     .topic1(multipool)
    //     .from_block(current_block - block_countback)
    //     .address(state.factory);
    // let events = state.provider.get_logs(&filter).await?;

    // let (fee_receiver, fee_amount) = events
    //     .into_iter()
    //     .find_map(
    //         |i| match MultipoolFactoryEvents::decode_log(&i.into()).ok()?.data {
    //             MultipoolFactoryEvents::ProtocolFeeSent(e) => Some((e.feeReceiver, e.amount)),
    //             _ => None,
    //         },
    //     )
    //     .ok_or(AppError::MultipoolNotCreated)?;

    if code.is_empty()
        && form.description.len() > 500
        && form.name.len() > 25
        && form.symbol.len() > 10
        && form.logo_bytes.len() > 1024 * 100
    {
        Err(AppError::InvalidPayloadSize)?;
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
    .bind(form.name)
    .bind(form.symbol)
    .bind(form.description)
    .bind(form.logo_bytes)
    .fetch_all(
        &mut *state
            .connection
            .acquire()
            .await
            .map_err(|_| AppError::DbIsBusy)?,
    )
    .await?;
    DATABASE_REQUEST_DURATION_MS.record(
        timer.elapsed().as_millis() as u64,
        &[KeyValue::new("query_name", "mp_dnl")],
    );

    Ok(().into())
    //let name_bytes = name.into_bytes();
    //let desc_offset = name_bytes.len();
    //let desc_bytes = description.into_bytes();
    //let logo_offset = desc_offset + desc_bytes.len();
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
}

//#[derive(Deserialize)]
//pub struct MetadataRequest {
//    multipool: Address,
//}

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
    //Path(multipool): Path<Address>,
    State(state): State<Arc<crate::AppState<P>>>,
) -> AppResult<MsgPack<Vec<DbMetadata>>> {
    sqlx::query_as("SELECT multipool, logo, description FROM multipools WHERE logo IS NOT NULL and description IS NOT NULL")
        //.bind::<[u8; 20]>(multipool.into())
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
    los: BigDecimal,
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
