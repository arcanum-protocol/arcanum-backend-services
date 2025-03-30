use alloy::primitives::{Address, U256};
use dashmap::DashMap;
use serde::Serialize;
use sqlx::PgPool;

pub struct MultipoolsCache {
    pub cache: DashMap<Address, MultipoolCache>,
}

impl MultipoolsCache {
    pub fn with_postgres(_pool: PgPool) -> Self {
        let cache = DashMap::default();
        Self { cache }
    }
}

#[derive(Serialize, Clone, Default)]
pub struct Candle {
    #[serde(rename(serialize = "t"))]
    time: u64,
    #[serde(rename(serialize = "o"))]
    open: U256,
    #[serde(rename(serialize = "c"))]
    close: U256,
    #[serde(rename(serialize = "lo"))]
    low: U256,
    #[serde(rename(serialize = "h"))]
    hight: U256,
}

#[derive(Serialize, Clone, Default)]
pub struct Stats {
    #[serde(rename(serialize = "l"))]
    low_24h: U256,
    #[serde(rename(serialize = "h"))]
    hight_24h: U256,
    #[serde(rename(serialize = "p"))]
    current_price: U256,
    #[serde(rename(serialize = "t"))]
    total_supply: U256,
    #[serde(rename(serialize = "pr"))]
    current_candle: Option<Candle>,
    #[serde(rename(serialize = "cu"))]
    previous_candle: Option<Candle>,
}

const RESOLUTION: usize = 1440;

pub struct MultipoolCache {
    daily_candles: [Option<Candle>; RESOLUTION],
    stats: Stats,
    last_queried_time: u64,
}

impl MultipoolCache {
    pub fn insert_total_supply(&mut self, total_supply: U256) {
        self.stats.total_supply = total_supply;
    }

    pub fn insert_candle(&mut self, candle: Candle) {
        self.daily_candles[candle.time as usize % RESOLUTION] = Some(candle.clone());
        self.stats.hight_24h = self
            .daily_candles
            .iter()
            .filter_map(|c| c.as_ref().map(|c| c.hight))
            .max()
            .map(|h| h.to_owned())
            .unwrap_or(U256::ZERO);
        self.stats.low_24h = self
            .daily_candles
            .iter()
            .filter_map(|c| c.as_ref().map(|c| c.low))
            .min()
            .map(|l| l.to_owned())
            .unwrap_or(U256::ZERO);

        self.stats.current_price = candle.close;
        self.stats.multipool_tvl = (self.stats.total_supply * candle.close) << 96;
        self.last_queried_time = candle.time;
        self.stats.previous_candle = self.stats.current_candle.replace(candle);
    }
}
