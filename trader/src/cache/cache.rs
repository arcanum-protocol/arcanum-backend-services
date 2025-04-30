use std::sync::RwLock;

use alloy::primitives::U256;
use alloy::{primitives::Address, providers::Provider};
use anyhow::Result;
use bigdecimal::BigDecimal;
use dashmap::DashMap;
use multipool::Multipool;
use serde::Serialize;
use serde::Serializer;
use std::sync::Arc;

use sqlx::{Executor, PgPool, Postgres};

pub struct Cache<P: Provider> {
    pub stats_cache: DashMap<Address, Multipool>,
    pub multipools: Arc<RwLock<Vec<Address>>>,
    pub connection: PgPool,
    pub provider: P,
    pub chain_id: u64,
    pub factory: Address,
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


impl<P: Provider> Cache<P> {
    pub async fn initialize(
        connection: PgPool,
        provider: P,
        factory: Address,
    ) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        let stats_cache = DashMap::<Address, Multipool>::default();
        let mut conn = connection.acquire().await?;

        let multipools = DbMultipool::get_with_chain_id(&mut *conn, chain_id).await?;

        for multipool in multipools.iter() {

        }

        let multipools = Arc::new(RwLock::new(
            multipools.into_iter().map(|m| m.multipool.into()).collect(),
        ));

        Ok(Self {
            factory,
            stats_cache,
            connection,
            provider,
            chain_id,
            multipools,
        })
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
