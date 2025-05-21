use alloy::primitives::Address;

use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;

pub struct AssetChange {
    total_supply: u128,
    multipool: Address,
}

impl AssetChange {
    pub fn new(total_supply: u128, multipool: Address) -> Self {
        Self {
            total_supply,
            multipool,
        }
    }

    pub fn get_query<'a>(self) -> Query<'a, Postgres, PgArguments> {
        sqlx::query(
            "
            UPDATE multipools
            SET total_supply = $1::NUMERIC
            WHERE multipool = $2;
        ",
        )
        .bind::<String>(self.total_supply.to_string())
        .bind::<[u8; 20]>(self.multipool.into())
    }
}
