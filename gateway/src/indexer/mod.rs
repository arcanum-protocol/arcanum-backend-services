use crate::service::log_target::GatewayTarget::Indexer;
use crate::service::metrics::{
    DATABASE_REQUEST_DURATION_MS, INDEXED_LOGS_COUNT, INDEXER_HEIGHT, LOGS_COMMITEMENT_DURATION_MS,
};
use alloy::primitives::{LogData, U256};
use alloy::{providers::Provider, sol_types::SolEventInterface};
use anyhow::{anyhow, Context, Result};
use asset_change::AssetChange;
use backend_service::logging::LogTarget;
use backend_service::KeyValue;
use create_pool::MultipoolCreated;
use indexer1::Processor;
use multipool_types::Multipool::MultipoolEvents;
use multipool_types::MultipoolFactory::MultipoolFactoryEvents;
use owner_change::OwnerChange;
use serde::{Deserialize, Serialize};
use serde_json::json;
use share_transfer::ShareTransfer;
use sqlx::Executor;
use sqlx::Postgres;
use std::sync::Arc;
use std::time::Instant;

pub mod asset_change;
pub mod create_pool;
pub mod owner_change;
pub mod share_transfer;

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

impl<'a, P: Provider + Clone> Processor<sqlx::Transaction<'a, Postgres>> for PgEventProcessor<P> {
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        db_tx: &mut sqlx::Transaction<'a, Postgres>,
        _prev_saved_block: u64,
        new_saved_block: u64,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        let commitement_timer = Instant::now();
        INDEXER_HEIGHT.record(new_saved_block, &[]);
        INDEXED_LOGS_COUNT.record(logs.len().try_into()?, &[]);

        let events = parse_logs(logs, &self.app_state.provider).await?;
        let mut queries = Vec::new();

        for event in events {
            //queries.push(
            //    sqlx::query(
            //        "INSERT INTO events(
            //        chain_id,
            //        block_number,
            //        transaction_index,
            //        event_index,
            //        payload
            //    ) VALUES ($1,$2,$3,$4,$5);",
            //    )
            //    .bind::<i64>(chain_id as i64)
            //    .bind::<i64>(event.block_number as i64)
            //    .bind::<i64>(event.transaction_index as i64)
            //    .bind::<i64>(event.event_index as i64)
            //    .bind::<Value>(to_value(&event)?),
            //);
            if let Ok(mp) = MultipoolFactoryEvents::decode_log(&event.event_data) {
                if let MultipoolFactoryEvents::MultipoolCreated(e) = mp.data {
                    Indexer
                        .info(json!({
                            "m": "New multipool found",
                            "a": e.multipoolAddress,
                        }))
                        .log();
                    if event.event_data.address == self.app_state.factory {
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
                    } else {
                        Indexer
                            .info(json!({
                                "m": "multipool created not by factory",
                                "a": event.event_data.address,
                            }))
                            .log();
                    }
                }
            }

            let multipool_address = event.event_data.address;
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
            if let Ok(multipool_event) = MultipoolEvents::decode_log(&event.event_data) {
                match multipool_event.data {
                    MultipoolEvents::ShareTransfer(e) => {
                        let price = self
                            .app_state
                            .stats_cache
                            .get(&multipool_address)
                            .expect("Multipool should present when having events")
                            .get_price(event.block_timestamp);
                        let price = match price {
                            Some(p) => p,
                            None => crate::price_fetcher::get_mps_prices(
                                &[multipool_address],
                                &self.app_state.provider,
                                event.block_number,
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
                            transaction_hash: event.transaction_hash,
                            block_number: event.block_number,
                            block_timestamp: event.block_timestamp,
                        };

                        if !e.from.is_zero() {
                            queries.push(share_transfer.get_query_sender());
                        }
                        if !e.to.is_zero() {
                            queries.push(share_transfer.get_query_receiver());
                        }
                    }
                    MultipoolEvents::MultipoolOwnerChange(e) => {
                        queries.push(OwnerChange::new(e.newOwner, multipool_address).get_query());
                    }
                    MultipoolEvents::AssetChange(e) => {
                        if e.asset == multipool_address {
                            queries.push(
                                AssetChange::new(e.quantity.to(), multipool_address).get_query(),
                            );
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

        let timer = Instant::now();
        for query in queries {
            db_tx.execute(query).await?;
        }
        DATABASE_REQUEST_DURATION_MS.record(
            timer.elapsed().as_millis() as u64,
            &[KeyValue::new("query_name", "mp_indexer_insertion")],
        );
        LOGS_COMMITEMENT_DURATION_MS.record(commitement_timer.elapsed().as_millis() as u64, &[]);
        Ok(())
    }
}

struct TimestampCache<'a, P: Provider> {
    provider: &'a P,
    block_number: u64,
    timestamp: u64,
}

impl<'a, P: Provider> TimestampCache<'a, P> {
    fn new(provider: &'a P) -> Self {
        Self {
            provider,
            block_number: 0,
            timestamp: 0,
        }
    }
    async fn extract(&mut self, log: &alloy::rpc::types::Log) -> Result<u64> {
        let block_number = log.block_number.context("block number is absent")?;

        if self.block_number == block_number {
            Ok(self.timestamp)
        } else {
            let ts = self
                .provider
                .get_block_by_number(block_number.into())
                .await?
                .map(|b| b.header.timestamp)
                .context("Block timestamp is absent in RPC")?;
            self.block_number = block_number;
            self.timestamp = ts;
            Ok(ts)
        }
    }
}

pub async fn parse_logs<P: Provider + Clone + 'static>(
    logs: &[alloy::rpc::types::Log],
    rpc: &P,
) -> anyhow::Result<Vec<Event>> {
    let mut cache = TimestampCache::new(&rpc);
    let mut events = Vec::new();

    for log in logs {
        events.push(Event {
            block_number: log.block_number.context("Block number is missing")?,
            block_hash: *log.block_hash.context("Block hash is missing")?,
            block_timestamp: cache.extract(&log).await?,
            transaction_hash: *log
                .transaction_hash
                .context("Transaction hash is missing")?,
            transaction_index: log
                .transaction_index
                .context("Transaction index is missing")?,
            event_index: log.log_index.context("Log index is missing")?,
            event_data: log.inner.clone(),
        });
    }
    Ok(events)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    #[serde(rename = "n")]
    pub block_number: u64,
    #[serde(rename = "h")]
    pub block_hash: [u8; 32],
    #[serde(rename = "t")]
    pub block_timestamp: u64,
    #[serde(rename = "ti")]
    pub transaction_index: u64,
    #[serde(rename = "th")]
    pub transaction_hash: [u8; 32],
    #[serde(rename = "ed")]
    pub event_data: alloy::primitives::Log<LogData>,
    #[serde(rename = "ei")]
    pub event_index: u64,
}
