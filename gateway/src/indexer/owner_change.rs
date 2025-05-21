use alloy::primitives::Address;

use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;

pub struct OwnerChange {
    new_owner: Address,
    multipool: Address,
}

impl OwnerChange {
    pub fn new(new_owner: Address, multipool: Address) -> Self {
        Self {
            new_owner,
            multipool,
        }
    }

    pub fn get_query<'a>(self) -> Query<'a, Postgres, PgArguments> {
        sqlx::query(
            "
            UPDATE multipools
            SET owner = $1
            WHERE multipool = $2;
        ",
        )
        .bind::<[u8; 20]>(self.new_owner.into())
        .bind::<[u8; 20]>(self.multipool.into())
    }
}
