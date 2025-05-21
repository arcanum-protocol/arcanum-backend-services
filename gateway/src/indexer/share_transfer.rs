use alloy::primitives::Address;

use bigdecimal::BigDecimal;
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;

pub struct ShareTransfer {
    pub chain_id: u64,
    pub multipool: Address,
    pub from: Address,
    pub to: Address,
    pub quantity: BigDecimal,
    pub quote_quantity: BigDecimal,
    pub transaction_hash: [u8; 32],
    pub block_number: u64,
    pub block_timestamp: u64,
}

impl ShareTransfer {
    const QUERY: &str = "INSERT INTO actions_history(
        chain_id,
        account,
        multipool,
        quantity,
        quote_quantity,
        transaction_hash,
        block_number,
        timestamp
    ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8);";

    pub fn get_query_sender<'a>(&self) -> Query<'a, Postgres, PgArguments> {
        sqlx::query(Self::QUERY)
            .bind::<i64>(self.chain_id as i64)
            .bind::<[u8; 20]>(self.from.into())
            .bind::<[u8; 20]>(self.multipool.into())
            .bind::<BigDecimal>(-self.quantity.clone())
            .bind::<BigDecimal>(-self.quote_quantity.clone())
            .bind::<[u8; 32]>(self.transaction_hash)
            .bind::<i64>(self.block_number as i64)
            .bind::<i64>(self.block_timestamp as i64)
    }

    pub fn get_query_receiver<'a>(&self) -> Query<'a, Postgres, PgArguments> {
        sqlx::query(Self::QUERY)
            .bind::<i64>(self.chain_id as i64)
            .bind::<[u8; 20]>(self.to.into())
            .bind::<[u8; 20]>(self.multipool.into())
            .bind::<BigDecimal>(self.quantity.clone())
            .bind::<BigDecimal>(self.quote_quantity.clone())
            .bind::<[u8; 32]>(self.transaction_hash)
            .bind::<i64>(self.block_number as i64)
            .bind::<i64>(self.block_timestamp as i64)
    }
}
