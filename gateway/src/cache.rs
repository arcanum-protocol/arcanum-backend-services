use std::sync::RwLock;

use alloy::{primitives::Address, providers::Provider};
use anyhow::Result;
use bigdecimal::BigDecimal;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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
}

impl DbCandle {
    async fn get_latest_day<'a, E: Executor<'a, Database = Postgres>>(
        executor: E,
    ) -> Result<Vec<Self>> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        sqlx::query_as("select * from candles where resolution = 60 and ts >= $1")
            .bind((current_time * 60 / 60 - RESOLUTION as u64) as i64)
            .fetch_all(executor)
            .await
            .map_err(Into::into)
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
                .or_default();
            for candle in candles
                .iter()
                .filter(|c| c.multipool == multipool.multipool)
            {
                e.insert_candle(Candle {
                    ts: candle.ts as u64,
                    open: candle.open.to_string().parse().unwrap(),
                    close: candle.close.to_string().parse().unwrap(),
                    low: candle.low.to_string().parse().unwrap(),
                    hight: candle.hight.to_string().parse().unwrap(),
                });
                e.insert_total_supply(multipool.total_supply.to_string().parse().unwrap())
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
    ts: u64,
    #[serde(rename(serialize = "o"))]
    open: u128,
    #[serde(rename(serialize = "c"))]
    close: u128,
    #[serde(rename(serialize = "l"))]
    low: u128,
    #[serde(rename(serialize = "h"))]
    hight: u128,
}

#[derive(Serialize, Clone, Default)]
pub struct Stats {
    #[serde(rename(serialize = "l24"))]
    low_24h: u128,
    #[serde(rename(serialize = "h24"))]
    hight_24h: u128,
    #[serde(rename(serialize = "p"))]
    current_price: u128,
    #[serde(rename(serialize = "ts"))]
    total_supply: u128,
    #[serde(rename(serialize = "cc"))]
    current_candle: Option<Candle>,
    #[serde(rename(serialize = "pc"))]
    previous_candle: Option<Candle>,
}

const RESOLUTION: usize = 1440;

pub struct MultipoolCache {
    daily_candles: [Option<Candle>; RESOLUTION],
    stats: Stats,
}

impl Default for MultipoolCache {
    fn default() -> Self {
        Self {
            daily_candles: [const { None }; RESOLUTION],
            stats: Default::default(),
        }
    }
}

impl MultipoolCache {
    pub fn insert_total_supply(&mut self, total_supply: u128) {
        self.stats.total_supply = total_supply;
    }

    pub fn get_price(&self, ts: u64) -> Option<u128> {
        self.daily_candles[(ts / 60 * 60) as usize % RESOLUTION]
            .as_ref()
            .map(|c| c.close)
    }

    // can be more optimised
    pub fn insert_price(&mut self, price: u128, ts: u64) {
        let c = self.daily_candles[(ts / 60 * 60) as usize % RESOLUTION]
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
        self.insert_candle(c);
    }

    // can be more optimised
    pub fn insert_candle(&mut self, candle: Candle) {
        self.daily_candles[candle.ts as usize % RESOLUTION] = Some(candle.clone());
        self.stats.hight_24h = self
            .daily_candles
            .iter()
            .filter_map(|c| c.as_ref().map(|c| c.hight))
            .max()
            .map(|h| h.to_owned())
            .unwrap_or(0);
        self.stats.low_24h = self
            .daily_candles
            .iter()
            .filter_map(|c| c.as_ref().map(|c| c.low))
            .min()
            .map(|l| l.to_owned())
            .unwrap_or(0);

        self.stats.current_price = candle.close;
        self.stats.previous_candle = self.stats.current_candle.replace(candle);
    }
}
