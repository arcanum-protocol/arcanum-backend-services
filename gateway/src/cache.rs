use std::sync::RwLock;

use alloy::primitives::U256;
use alloy::{primitives::Address, providers::Provider};
use anyhow::Result;
use arweave_client::{Rpc, Signer};
use bigdecimal::BigDecimal;
use dashmap::DashMap;
use serde::Serialize;
use serde::Serializer;
use std::sync::Arc;

use sqlx::{Executor, PgPool, Postgres};

pub struct AppState<P: Provider> {
    pub stats_cache: DashMap<Address, MultipoolCache>,
    pub multipools: Arc<RwLock<Vec<Address>>>,
    pub connection: PgPool,
    pub arweave_rpc: Rpc,
    pub arweave_signer: Arc<Signer>,
    pub provider: P,
    pub chain_id: u64,
    pub factory: Address,
}

#[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
struct DbMultipool {
    multipool: [u8; 20],
    total_supply: BigDecimal,
    owner: [u8; 20],
    name: String,
    symbol: String,
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

#[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct DbCandle {
    ts: i64,
    open: BigDecimal,
    close: BigDecimal,
    low: BigDecimal,
    hight: BigDecimal,
    multipool: [u8; 20],
    resolution: i32,
}

#[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct DbCandleSmall {
    t: i64,
    o: BigDecimal,
    c: BigDecimal,
    l: BigDecimal,
    h: BigDecimal,
}

impl From<DbCandleSmall> for Candle {
    fn from(value: DbCandleSmall) -> Self {
        Self {
            ts: value.t as u64,
            open: value.o.to_string().parse().unwrap(),
            close: value.c.to_string().parse().unwrap(),
            low: value.l.to_string().parse().unwrap(),
            hight: value.h.to_string().parse().unwrap(),
        }
    }
}

impl DbCandle {
    async fn get_latest_day<E>(executor: &mut E) -> Result<Vec<Self>>
    where
        for<'b> &'b mut E: Executor<'b, Database = Postgres>,
    {
        let mut candles = Vec::default();
        for resolution in RESOLUTIONS {
            let part = sqlx::query_as(
                "select * from candles WHERE resolution = $1 ORDER BY ts DESC LIMIT $2",
            )
            .bind(resolution as i32)
            .bind(MAX_BUFFER_SIZE as i64)
            .fetch_all(&mut *executor)
            .await?;
            candles.extend(part);
        }
        Ok(candles)
    }
}

impl<P: Provider> AppState<P> {
    pub async fn initialize(
        connection: PgPool,
        provider: P,
        factory: Address,
        arweave_url: String,
        wallet_path: &str,
    ) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        let stats_cache = DashMap::<Address, MultipoolCache>::default();
        let mut conn = connection.acquire().await?;

        let multipools = DbMultipool::get_with_chain_id(&mut *conn, chain_id).await?;
        let candles = DbCandle::get_latest_day(&mut *conn).await?;

        for multipool in multipools.iter() {
            let mut e = stats_cache
                .entry(Address::new(multipool.multipool))
                .or_insert(MultipoolCache::new(
                    multipool.name.clone(),
                    multipool.symbol.clone(),
                ));
            e.insert_total_supply(multipool.total_supply.to_string().parse().unwrap());
            for candle in candles
                .iter()
                .filter(|c| c.multipool == multipool.multipool)
            {
                e.insert_price(candle.open.to_string().parse().unwrap(), candle.ts as u64);
                e.insert_price(candle.low.to_string().parse().unwrap(), candle.ts as u64);
                e.insert_price(candle.hight.to_string().parse().unwrap(), candle.ts as u64);
                e.insert_price(candle.close.to_string().parse().unwrap(), candle.ts as u64);
            }
        }

        let multipools = Arc::new(RwLock::new(
            multipools.into_iter().map(|m| m.multipool.into()).collect(),
        ));

        Ok(Self {
            arweave_rpc: Rpc {
                url: arweave_url,
                ..Default::default()
            },
            arweave_signer: Arc::new(Signer::from_file(wallet_path)?),
            factory,
            stats_cache,
            connection,
            provider,
            chain_id,
            multipools,
        })
    }
}

#[derive(Serialize, Clone, Default)]
pub struct Candle {
    #[serde(rename(serialize = "t"))]
    #[serde(serialize_with = "serialize_u64")]
    pub ts: u64,
    #[serde(rename(serialize = "o"))]
    #[serde(serialize_with = "serialize_u256")]
    pub open: U256,
    #[serde(rename(serialize = "c"))]
    #[serde(serialize_with = "serialize_u256")]
    pub close: U256,
    #[serde(rename(serialize = "l"))]
    #[serde(serialize_with = "serialize_u256")]
    pub low: U256,
    #[serde(rename(serialize = "h"))]
    #[serde(serialize_with = "serialize_u256")]
    pub hight: U256,
}

#[derive(Serialize, Clone, Default)]
pub struct Stats {
    #[serde(rename(serialize = "n"))]
    name: String,
    #[serde(rename(serialize = "s"))]
    symbol: String,
    #[serde(rename(serialize = "l"))]
    #[serde(serialize_with = "serialize_u256")]
    low_24h: U256,
    #[serde(rename(serialize = "h"))]
    #[serde(serialize_with = "serialize_u256")]
    hight_24h: U256,
    #[serde(rename(serialize = "c"))]
    #[serde(serialize_with = "serialize_u256")]
    current_price: U256,
    #[serde(rename(serialize = "o"))]
    #[serde(serialize_with = "serialize_u256")]
    open_price: U256,
    #[serde(rename(serialize = "t"))]
    #[serde(serialize_with = "serialize_u128")]
    total_supply: u128,
    #[serde(rename(serialize = "cc"))]
    current_candle: Option<Candle>,
    #[serde(rename(serialize = "pc"))]
    previous_candle: Option<Candle>,
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

pub const MAX_BUFFER_SIZE: usize = 96;
pub const TRW_RESOLUTION: usize = 1;
pub const DAY: i32 = 86400;
pub const MINUTE: i32 = 60;

pub const RESOLUTIONS: [i32; 4] = [MINUTE, 900, 3600, DAY];

pub fn resolution_to_index(resolution: i32) -> usize {
    try_resolution_to_index(resolution).unwrap()
}

pub fn try_resolution_to_index(resolution: i32) -> Option<usize> {
    RESOLUTIONS
        .into_iter()
        .position(|r| r.eq(&resolution))
        .map(TryInto::try_into)
        .transpose()
        .unwrap()
}

pub fn index_to_resolution(index: usize) -> i32 {
    RESOLUTIONS[index]
}

pub struct MultipoolCache {
    pub candles: [Vec<Candle>; 4],
    pub trw_start_index: usize,
    pub stats: Stats,
}

impl MultipoolCache {
    pub fn new(name: String, symbol: String) -> Self {
        Self {
            candles: Default::default(),
            trw_start_index: 0,
            stats: Stats {
                name,
                symbol,
                ..Default::default()
            },
        }
    }

    pub fn insert_total_supply(&mut self, total_supply: u128) {
        self.stats.total_supply = total_supply;
    }

    fn align_by(ts: u64, resolution: i32) -> u64 {
        ts / resolution as u64 * resolution as u64
    }

    pub fn get_price(&self, ts: u64) -> Option<U256> {
        self.get_candle(ts, MINUTE).map(|c| c.close)
    }

    fn get_candle(&self, ts: u64, resolution: i32) -> Option<Candle> {
        self.candles[resolution_to_index(resolution)]
            .iter()
            .rev()
            .find(|c| c.ts == Self::align_by(ts, resolution))
            .cloned()
    }

    //TODO: tests (dis shit is crazy)
    pub fn insert_price(&mut self, price: U256, ts: u64) {
        for resolution in RESOLUTIONS {
            let resolution_index = resolution_to_index(resolution);

            let candle = self
                .get_candle(ts, resolution)
                .map(|mut c| {
                    c.hight = c.hight.max(price);
                    c.low = c.low.min(price);
                    c.close = price;
                    c
                })
                .unwrap_or(Candle {
                    ts,
                    open: price,
                    close: price,
                    low: price,
                    hight: price,
                });

            self.candles[resolution_index].push(candle.clone());

            if resolution_index == TRW_RESOLUTION {
                self.trw_start_index = self.candles[resolution_index][self.trw_start_index..]
                    .iter()
                    .position(|c| candle.ts > c.ts + DAY as u64)
                    .map(|n| self.trw_start_index + n)
                    .unwrap_or(self.candles[resolution_index].len() - 1);

                let trw_iter = self.candles[TRW_RESOLUTION][self.trw_start_index..].iter();
                self.stats.hight_24h = trw_iter.clone().map(|c| c.hight).max().unwrap_or_default();
                self.stats.low_24h = trw_iter.map(|c| c.low).min().unwrap_or_default();

                self.stats.open_price = self.candles[TRW_RESOLUTION][self.trw_start_index].open;
                self.stats.current_price = candle.close;

                // doesnt work somehow
                match self.stats.current_candle {
                    Some(ref c) if c.ts < candle.ts => {
                        self.stats.previous_candle = self.stats.current_candle.replace(candle);
                    }
                    None => {
                        self.stats.current_candle = Some(candle);
                    }
                    _ => (),
                }
            }

            let buf_len = self.candles[resolution_index].len();
            self.candles[resolution_index]
                .rotate_left(buf_len.checked_sub(MAX_BUFFER_SIZE).unwrap_or_default());
            self.candles[resolution_index].truncate(MAX_BUFFER_SIZE);
        }
    }
}
