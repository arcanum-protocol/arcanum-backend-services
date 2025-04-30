use std::sync::RwLock;

use alloy::primitives::U256;
use alloy::{primitives::Address, providers::Provider};
use anyhow::Result;
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
struct DbCandle {
    ts: i64,
    open: BigDecimal,
    close: BigDecimal,
    low: BigDecimal,
    hight: BigDecimal,
    multipool: [u8; 20],
    resolution: i32,
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
            .bind(BUFFER_SIZE as i64)
            .fetch_all(&mut *executor)
            .await?;
            candles.extend(part);
        }
        Ok(candles)
    }
}

impl<P: Provider> AppState<P> {
    pub async fn initialize(connection: PgPool, provider: P, factory: Address) -> Result<Self> {
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
                e.insert_candle(
                    candle.resolution,
                    Candle {
                        ts: candle.ts as u64,
                        open: candle.open.to_string().parse().unwrap(),
                        close: candle.close.to_string().parse().unwrap(),
                        low: candle.low.to_string().parse().unwrap(),
                        hight: candle.hight.to_string().parse().unwrap(),
                    },
                );
            }
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

pub struct MultipoolCache {
    pub candles: Box<[[Option<Candle>; BUFFER_SIZE]; 4]>,
    pub stats: Stats,
}

impl MultipoolCache {
    pub fn new(name: String, symbol: String) -> Self {
        Self {
            candles: Box::new([const { [const { None }; BUFFER_SIZE] }; 4]),
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

    pub fn get_price(&self, ts: u64) -> Option<U256> {
        self.candles[0][(ts / 60) as usize % BUFFER_SIZE]
            .as_ref()
            .map(|c| c.close)
    }

    // can be more optimised
    pub fn insert_price(&mut self, price: U256, ts: u64) {
        for r in RESOLUTIONS {
            let c = self.candles[resolution_to_index(r)][(ts / r as u64) as usize % BUFFER_SIZE]
                .clone()
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
            let mut c = c.clone();
            c.ts = c.ts / r as u64 * r as u64;
            self.insert_candle(r, c);
        }
    }

    // can be more optimised
    pub fn insert_candle(&mut self, resolution: i32, candle: Candle) {
        let resolution_index = resolution_to_index(resolution);
        self.candles[resolution_index][(candle.ts / resolution as u64) as usize % BUFFER_SIZE] =
            Some(candle.clone());

        if resolution_index == TRW_RESOLUTION {
            //NOTICE: search of open can be optimised by knowing where is the oldest or newest element
            self.stats.open_price = self.candles[TRW_RESOLUTION]
                .iter()
                .filter_map(|v| v.as_ref())
                .min_by_key(|c| c.ts)
                .map(|c| c.open)
                .unwrap_or_default();
            self.stats.hight_24h = self.candles[TRW_RESOLUTION]
                .iter()
                .filter_map(|c| c.as_ref().map(|c| c.hight))
                .max()
                .map(|h| h.to_owned())
                .unwrap_or_default();
            self.stats.low_24h = self.candles[TRW_RESOLUTION]
                .iter()
                .filter_map(|c| c.as_ref().map(|c| c.low))
                .min()
                .map(|l| l.to_owned())
                .unwrap_or_default();

            self.stats.current_price = candle.close;
            match self.stats.current_candle {
                Some(ref c) if c.ts < candle.ts => {
                    self.stats.previous_candle = self.stats.current_candle.replace(candle);
                }
                _ => (),
            }
        }
    }
}
