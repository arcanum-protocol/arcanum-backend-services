use alloy::{hex::ToHexExt, primitives::U256};
use multipool_storage::hook::HookInitializer;
use rdkafka::producer::FutureProducer;
use sqlx::{types::BigDecimal, PgPool};
use std::{ops::Div, str::FromStr, time::Duration};

#[derive(Clone)]
pub struct Etl {
    pub producer: FutureProducer,
    pub delay: Duration,
    //TOOD: in a lot of places chain id can be gained from RPC
    pub chain_id: u64,
    pub pool: PgPool,
}

impl HookInitializer for Etl {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let chain_id = self.chain_id.clone();
        let instance = self.clone();
        vec![tokio::spawn(async move {
            loop {
                let mp = multipool();

                let price = mp.get_price(&mp.contract_address()).unwrap();
                //todo can fail: retry

                let value = price.clone().any_age();
                let timestamp = price.time();

                while sqlx::query("call insert_price($1, $2, $3, $4);")
                    .bind::<i64>(chain_id as i64)
                    .bind::<String>(format!("\\x{}", mp.contract_address().encode_hex()))
                    .bind::<i64>(timestamp as i64)
                    .bind::<BigDecimal>(
                        BigDecimal::from_str(value.to_string().as_str())
                            .unwrap()
                            .div(
                                BigDecimal::from_str(
                                    U256::from(2).pow(U256::from(96)).to_string().as_str(),
                                )
                                .unwrap(),
                            ),
                    )
                    .execute(&mut *instance.pool.acquire().await?)
                    .await
                    .is_err()
                {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                // POSTGRES
                tokio::time::sleep(instance.delay).await;
            }
        })]
    }
}
