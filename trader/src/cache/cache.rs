use std::sync::RwLock;
use std::time::Duration;

use crate::cache::multipool::Multipool;
use crate::Click;
use alloy::primitives::U256;
use alloy::{primitives::Address, providers::Provider};
use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use dashmap::DashMap;
use serde::Serialize;
use serde::Serializer;
use std::sync::Arc;
use tokio::task::JoinSet;

use sqlx::{Executor, PgPool, Postgres};

pub struct Cache<P: Provider> {
    pub mp_cache: DashMap<Address, Multipool>,
    pub connection: PgPool,
    pub provider: Arc<P>,
    pub chain_id: u64,
}

#[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
struct DbMultipool {
    multipool: [u8; 20],
    total_supply: BigDecimal,
}

impl DbMultipool {
    async fn get_with_chain_id<'a, E: Executor<'a, Database = Postgres>>(
        executor: E,
        chain_id: u64,
    ) -> Result<Vec<Self>> {
        sqlx::query_as("select * from multipools where chain_id = $1")
            .bind(chain_id as i64)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
    }
}

impl<P: Provider + Sync + Send + 'static> Cache<P> {
    pub async fn initialize(connection: PgPool, provider: Arc<P>) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        let mut conn = connection.acquire().await?;

        let multipools = DbMultipool::get_with_chain_id(&mut *conn, chain_id).await?;
        let mut set = JoinSet::new();
        for multipool in multipools.iter() {
            let p = provider.clone();
            let address = Address::from(multipool.multipool);
            set.spawn(Multipool::from_rpc(p, address));
        }
        let mp_cache = set
            .join_all()
            .await
            .into_iter()
            .filter_map(|res| {res.ok().map(|m| (m.address, m))})
            .collect();

        Ok(Self {
            mp_cache,
            connection,
            provider,
            chain_id,
        })
    }
    pub async fn update(&self) -> Result<()> {
        let mut conn = self.connection.acquire().await?;

        let multipools = DbMultipool::get_with_chain_id(&mut *conn, self.chain_id).await?;
        let mut set = JoinSet::new();
        for multipool in multipools.iter() {
            let p = self.provider.clone();
            let address = Address::from(multipool.multipool);
            set.spawn(Multipool::from_rpc(p, address));
        }
        let mp_cache: Vec<Multipool> = set
            .join_all()
            .await
            .into_iter()
            .filter_map(|res| {res.ok()})
            .collect();
        for mp in mp_cache.into_iter() {
            self.mp_cache.insert(mp.address, mp);
        }
        Ok(())
    }
}

pub fn serialize_u256<S>(number: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Convert byte array to Address
    // Serialize as a hex string with 0x prefix
    serializer.serialize_str(&number.to_string())
}

pub fn serialize_u128<S>(number: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Convert byte array to Address
    // Serialize as a hex string with 0x prefix
    serializer.serialize_str(&number.to_string())
}

pub fn serialize_u64<S>(number: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Convert byte array to Address
    // Serialize as a hex string with 0x prefix
    serializer.serialize_str(&number.to_string())
}

pub const BUFFER_SIZE: usize = 96;
pub const TRW_RESOLUTION: usize = 1;

pub const RESOLUTIONS: [i32; 4] = [60, 900, 3600, 86400];

pub fn resolution_to_index(resolution: i32) -> usize {
    RESOLUTIONS
        .into_iter()
        .position(|r| r.eq(&resolution))
        .unwrap() as usize
}

pub fn index_to_resolution(index: usize) -> i32 {
    RESOLUTIONS[index]
}
