use alloy::primitives::{I256, U256};
use anyhow::{Context, Result};
use clickhouse::{Client, Row};
use serde::Serialize;

const TABLE_NAME: &str = "trades";

pub struct Click {
    client: Client,
}

impl Click {
    pub fn new() -> Result<Self> {
        let url = std::env::var("CLICKHOUSE_URL").context("Clickhouse url is not provided")?;
        let user = std::env::var("CLICKHOUSE_USER").context("Clickhouse user is not provided")?;
        let pass =
            std::env::var("CLICKHOUSE_PASSWORD").context("Clickhouse password is not provided")?;
        let db = std::env::var("CLICKHOUSE_DB").context("Clickhouse password is not provided")?;
        let client = Client::default()
            .with_url(url)
            .with_user(user)
            .with_password(pass)
            .with_database(db);
        Ok(Self { client })
    }

    pub async fn insert(&self, stats: TradeStats) -> Result<()> {
        let mut statement = self.client.insert(TABLE_NAME)?;
        statement.write(&stats).await?;
        statement.end().await?;
        Ok(())
    }
}

#[derive(Row, Serialize)]
pub struct TradeStats {
    pub multipool_address: String,
    #[serde(with = "u256")]
    pub trade_input: U256,
    #[serde(with = "u256")]
    pub trade_output: U256,
    #[serde(with = "i256")]
    pub multipool_fee: I256,
    pub asset_in_address: String,
    pub asset_out_address: String,
    pub pool_in_address: String,
    pub pool_in_fee: u32,
    pub pool_out_address: String,
    pub pool_out_fee: u32,
    #[serde(with = "u256")]
    pub multipool_amount_in: U256,
    #[serde(with = "u256")]
    pub multipool_amount_out: U256,
    pub strategy_type: String,
}

// u256 serde -- https://github.com/ClickHouse/clickhouse-rs/issues/48
pub mod u256 {
    use alloy::primitives::U256;
    use serde::{
        de::{Deserialize, Deserializer},
        ser::{Serialize, Serializer},
    };

    /// evm U256 is represented in big-endian, but ClickHouse expects little-endian
    pub fn serialize<S: Serializer>(u: &U256, serializer: S) -> Result<S::Ok, S::Error> {
        let buf: [u8; 32] = u.to_le_bytes();
        buf.serialize(serializer)
    }

    /// ClickHouse stores U256 in little-endian
    pub fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf: [u8; 32] = Deserialize::deserialize(deserializer)?;
        Ok(U256::from_le_bytes(buf))
    }
}

// i256 serde -- https://github.com/ClickHouse/clickhouse-rs/issues/48
pub mod i256 {
    use alloy::primitives::I256;
    use serde::{
        de::{Deserialize, Deserializer},
        ser::{Serialize, Serializer},
    };

    /// evm I256 is represented in big-endian, but ClickHouse expects little-endian
    pub fn serialize<S: Serializer>(u: &I256, serializer: S) -> Result<S::Ok, S::Error> {
        let buf: [u8; 32] = u.to_le_bytes();
        buf.serialize(serializer)
    }

    /// ClickHouse stores I256 in little-endian
    pub fn deserialize<'de, D>(deserializer: D) -> Result<I256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf: [u8; 32] = Deserialize::deserialize(deserializer)?;
        Ok(I256::from_le_bytes(buf))
    }
}
