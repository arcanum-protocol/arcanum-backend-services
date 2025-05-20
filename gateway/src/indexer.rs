use crate::service::log_target::GatewayTarget::Indexer;
use crate::service::metrics::{
    DATABASE_REQUEST_DURATION_MS, INDEXED_LOGS_COUNT, INDEXER_HEIGHT, LOGS_COMMITEMENT_DURATION_MS,
};
use alloy::primitives::{Address, U256};
use alloy::{providers::Provider, sol_types::SolEventInterface};
use anyhow::{anyhow, Result};
use backend_service::logging::LogTarget;
use backend_service::KeyValue;
use indexer1::Processor;
use multipool_types::messages::Blocks;
use multipool_types::Multipool::MultipoolEvents;
use multipool_types::MultipoolFactory::MultipoolFactoryEvents;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::{json, to_value};
use sqlx::Executor;
use sqlx::{types::BigDecimal, Postgres, Transaction};
use std::sync::Arc;
use std::time::Instant;

use crate::cache::{AppState, MultipoolCache};

#[derive(Serialize, Deserialize, Debug)]
pub struct TradingAction {
    account: String,
    multipool: String,
    chain_id: i64,
    action_type: String,
    quantity: String,
    quote_quantity: Option<String>,
    transaction_hash: String,
    timestamp: i64,
}

#[derive(Deserialize)]
pub struct IndexerConfig {
    pub from_block: u64,
    pub fetch_interval_ms: u64,
    pub overtake_interval_ms: u64,
    pub max_block_range: Option<u64>,
}

pub struct PgEventProcessor<P: Provider + Clone + 'static> {
    pub app_state: Arc<AppState<P>>,
}

impl<'a, P: Provider + Clone> Processor<Transaction<'a, Postgres>> for PgEventProcessor<P> {
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        db_tx: &mut Transaction<'a, Postgres>,
        _prev_saved_block: u64,
        new_saved_block: u64,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        let commitement_timer = Instant::now();
        INDEXER_HEIGHT.record(new_saved_block, &[]);
        INDEXED_LOGS_COUNT.record(logs.len().try_into()?, &[]);

        let blocks = Blocks::parse_logs(logs, self.app_state.provider.clone())
            .await
            .map_err(|_e| anyhow!("ParseLogsErrror"))?;

        for block in blocks.0.iter() {
            let timer = Instant::now();
            sqlx::query(
                "INSERT INTO blocks(
                    chain_id,
                    block_number, payload
                ) VALUES ($1,$2,$3);",
            )
            .bind::<i64>(chain_id as i64)
            .bind::<i64>(block.number as i64)
            .bind::<Value>(to_value(block)?)
            .execute(&mut **db_tx)
            .await?;
            DATABASE_REQUEST_DURATION_MS.record(
                timer.elapsed().as_millis() as u64,
                &[KeyValue::new("query_name", "insert_blocks")],
            );

            for transaction in block.transactions.iter() {
                for event in transaction.events.iter() {
                    if let Ok(mp) = MultipoolFactoryEvents::decode_log(&event.log) {
                        if let MultipoolFactoryEvents::MultipoolCreated(e) = mp.data {
                            Indexer
                                .info(json!({
                                    "m": "New multipool found",
                                    "a": e.multipoolAddress,
                                }))
                                .log();
                            if event.log.address != self.app_state.factory {
                                Indexer
                                    .info(json!({
                                        "m": "multipool created not by factory",
                                        "a": event.log.address,
                                    }))
                                    .log();
                                continue;
                            }
                            MultipoolCreated::new(
                                e.multipoolAddress,
                                chain_id,
                                e.name.clone(),
                                e.symbol.clone(),
                            )
                            .apply_on_storage(&mut **db_tx)
                            .await?;

                            self.app_state
                                .stats_cache
                                .insert(e.multipoolAddress, MultipoolCache::new(e.name, e.symbol));
                            let mut multipools = self.app_state.multipools.write().unwrap();
                            multipools.push(e.multipoolAddress);
                        }
                    }

                    let multipool_address = event.log.address;
                    if self.app_state.stats_cache.get(&multipool_address).is_none() {
                        //TODO: FACTORY EVENTS ALSO GET HERE
                        Indexer
                            .info(json!({
                                "m": "multipool event is orphan, skipping",
                                "a": multipool_address,
                            }))
                            .log();
                        continue;
                    }

                    if let Ok(multipool_event) = MultipoolEvents::decode_log(&event.log) {
                        match multipool_event.data {
                            MultipoolEvents::ShareTransfer(e) => {
                                let price = self
                                    .app_state
                                    .stats_cache
                                    .get(&multipool_address)
                                    .expect("Multipool should present when having events")
                                    .get_price(block.timestamp);
                                let price = match price {
                                    Some(p) => p,
                                    None => crate::price_fetcher::get_mps_prices(
                                        &[multipool_address],
                                        &self.app_state.provider,
                                        block.number,
                                    )
                                    .await?
                                    .0[0]
                                        .ok_or(anyhow!("Price is not returned from chain"))?,
                                };
                                // TODO: if price is missing - push it into cache and db also

                                let quote_quantity: U256 = (e.amount * price) >> 96;

                                let share_transfer = ShareTransfer {
                                    chain_id,
                                    multipool: multipool_address,
                                    from: e.from,
                                    to: e.to,
                                    quantity: e.amount.to_string().parse().unwrap(),
                                    quote_quantity: quote_quantity.to_string().parse().unwrap(),
                                    transaction_hash: transaction.hash,
                                    block_number: block.number,
                                    block_timestamp: block.timestamp,
                                };

                                if !e.from.is_zero() {
                                    share_transfer
                                        .apply_on_storage_for_sender(&mut **db_tx)
                                        .await?;
                                }
                                if !e.to.is_zero() {
                                    share_transfer
                                        .apply_on_storage_for_receiver(&mut **db_tx)
                                        .await?;
                                }
                            }
                            MultipoolEvents::MultipoolOwnerChange(e) => {
                                OwnerChange::new(e.newOwner, multipool_address)
                                    .apply_on_storage(&mut **db_tx)
                                    .await?;
                            }
                            MultipoolEvents::AssetChange(e) => {
                                if e.asset == multipool_address {
                                    AssetChange::new(e.quantity.to(), multipool_address)
                                        .apply_on_storage(&mut **db_tx)
                                        .await?;
                                    self.app_state
                                        .stats_cache
                                        .get_mut(&e.asset)
                                        .expect("Multipool should present when having events")
                                        .insert_total_supply(e.quantity.to());
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
        LOGS_COMMITEMENT_DURATION_MS.record(commitement_timer.elapsed().as_millis() as u64, &[]);
        Ok(())
    }
}

pub struct MultipoolCreated {
    name: String,
    symbol: String,
    multipool: Address,
    chain_id: u64,
}

impl MultipoolCreated {
    fn new(multipool: Address, chain_id: u64, name: String, symbol: String) -> Self {
        Self {
            multipool,
            chain_id,
            name,
            symbol,
        }
    }

    async fn apply_on_storage<'a, E: Executor<'a, Database = Postgres>>(
        self,
        executor: E,
    ) -> Result<()> {
        let timer = Instant::now();
        let r = sqlx::query(
            "INSERT INTO multipools(
                chain_id,
                multipool,
                name,
                symbol,
                owner
            ) VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT (multipool) DO UPDATE
            SET
                name = $3,
                symbol = $4;
            ",
        )
        .bind::<i64>(self.chain_id.try_into()?)
        .bind::<[u8; 20]>(self.multipool.into())
        .bind::<String>(self.name)
        .bind::<String>(self.symbol)
        .bind::<[u8; 20]>(Address::ZERO.into())
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(Into::into);
        DATABASE_REQUEST_DURATION_MS.record(
            timer.elapsed().as_millis() as u64,
            &[KeyValue::new("query_name", "insert_multipool")],
        );
        r
    }
}

pub struct OwnerChange {
    new_owner: Address,
    multipool: Address,
}

impl OwnerChange {
    fn new(new_owner: Address, multipool: Address) -> Self {
        Self {
            new_owner,
            multipool,
        }
    }

    async fn apply_on_storage<'a, E: Executor<'a, Database = Postgres>>(
        self,
        executor: E,
    ) -> Result<()> {
        let timer = Instant::now();
        let r = sqlx::query(
            "
            UPDATE multipools
            SET owner = $1
            WHERE multipool = $2;
        ",
        )
        .bind::<[u8; 20]>(self.new_owner.into())
        .bind::<[u8; 20]>(self.multipool.into())
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(Into::into);
        DATABASE_REQUEST_DURATION_MS.record(
            timer.elapsed().as_millis() as u64,
            &[KeyValue::new("query_name", "update_mp_owner")],
        );
        r
    }
}

pub struct AssetChange {
    total_supply: u128,
    multipool: Address,
}

impl AssetChange {
    fn new(total_supply: u128, multipool: Address) -> Self {
        Self {
            total_supply,
            multipool,
        }
    }

    async fn apply_on_storage<'a, E: Executor<'a, Database = Postgres>>(
        self,
        executor: E,
    ) -> Result<()> {
        let timer = Instant::now();
        let r = sqlx::query(
            "
            UPDATE multipools
            SET total_supply = $1::NUMERIC
            WHERE multipool = $2;
        ",
        )
        .bind::<String>(self.total_supply.to_string())
        .bind::<[u8; 20]>(self.multipool.into())
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(Into::into);
        DATABASE_REQUEST_DURATION_MS.record(
            timer.elapsed().as_millis() as u64,
            &[KeyValue::new("query_name", "update_mp_ts")],
        );
        r
    }
}

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

    async fn apply_on_storage_for_sender<'a, E: Executor<'a, Database = Postgres>>(
        &self,
        executor: E,
    ) -> Result<()> {
        let timer = Instant::now();
        let r = sqlx::query(Self::QUERY)
            .bind::<i64>(self.chain_id as i64)
            .bind::<[u8; 20]>(self.from.into())
            .bind::<[u8; 20]>(self.multipool.into())
            .bind::<BigDecimal>(-self.quantity.clone())
            .bind::<BigDecimal>(-self.quote_quantity.clone())
            .bind::<[u8; 32]>(self.transaction_hash)
            .bind::<i64>(self.block_number as i64)
            .bind::<i64>(self.block_timestamp as i64)
            .execute(executor)
            .await
            .map(|_| ())
            .map_err(Into::into);
        DATABASE_REQUEST_DURATION_MS.record(
            timer.elapsed().as_millis() as u64,
            &[KeyValue::new("query_name", "insert_action")],
        );
        r
    }

    async fn apply_on_storage_for_receiver<'a, E: Executor<'a, Database = Postgres>>(
        &self,
        executor: E,
    ) -> Result<()> {
        let timer = Instant::now();
        let r = sqlx::query(Self::QUERY)
            .bind::<i64>(self.chain_id as i64)
            .bind::<[u8; 20]>(self.to.into())
            .bind::<[u8; 20]>(self.multipool.into())
            .bind::<BigDecimal>(self.quantity.clone())
            .bind::<BigDecimal>(self.quote_quantity.clone())
            .bind::<[u8; 32]>(self.transaction_hash)
            .bind::<i64>(self.block_number as i64)
            .bind::<i64>(self.block_timestamp as i64)
            .execute(executor)
            .await
            .map(|_| ())
            .map_err(Into::into);
        DATABASE_REQUEST_DURATION_MS.record(
            timer.elapsed().as_millis() as u64,
            &[KeyValue::new("query_name", "insert_action")],
        );
        r
    }
}
