use alloy::hex::ToHexExt;
use alloy::{
    primitives::Address,
    providers::Provider,
    sol_types::{SolEventInterface, SolValue},
};
use anyhow::anyhow;
use indexer1::Processor;
use multipool_storage::storage::MultipoolsCreation;
use multipool_types::messages::Blocks;
use multipool_types::Multipool::MultipoolEvents;
use multipool_types::MultipoolFactory::MultipoolFactoryEvents;
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use serde_json::Value;
use sqlx::{types::BigDecimal, Acquire, Postgres, Transaction};
use std::sync::Arc;

use std::time::Duration;

use crate::cache::AppState;

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

pub struct PgEventProcessor<P: Provider + Clone + 'static> {
    pub app_state: Arc<AppState<P>>,
}

impl<P: Provider + Clone> Processor<Transaction<'_, Postgres>> for PgEventProcessor<P> {
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        db_tx: &mut Transaction<'_, Postgres>,
        _prev_saved_block: u64,
        _new_saved_block: u64,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        //Add new pools
        //Update total supply
        //Update owner
        //
        // IN DB + state
        // Get prices to transfers (use cache to check wether to wait)
        //
        // Calculate pnl procedure
        // Store raw jsonb blocks

        let blocks = Blocks::parse_logs(logs, self.app_state.provider.clone())
            .await
            .map_err(|_e| anyhow!("ParseLogsErrror"))?;

        let creations: MultipoolsCreation = blocks.0.as_slice().try_into()?;
        for creation in creations.0.into_iter() {
            //NAME
            //SYMBOL from contracts
            sqlx::query(
                "INSERT INTO multipools(
                    chain_id,
                    multipool,
                    owner
                ) VALUES ($1, $2, $3);",
            )
            .bind::<i64>(chain_id.try_into()?)
            .bind::<&[u8]>(creation.multipool_address.as_slice())
            .bind::<&[u8]>(creation.address.as_slice())
            .execute(&mut **db_tx)
            .await?;
        }

        for block in blocks.0.iter() {
            sqlx::query(
                "INSERT INTO blocks(
                    chain_id, 
                    block_number,
                    payload
                ) VALUES ($1,$2,$3);",
            )
            .bind::<i64>(chain_id as i64)
            .bind::<i64>(block.number as i64)
            .bind::<Value>(to_value(block)?)
            .execute(&mut **db_tx)
            .await?;

            for transaction in block.transactions.iter() {
                for event in transaction.events.iter() {
                    // match total supply
                    // match share transfer
                    // match ownership transfer
                    if let Ok(multipool_event) = MultipoolEvents::decode_log(&event.log, false) {
                        match multipool_event.data {
                            MultipoolEvents::ShareTransfer(e) => {
                                let amount: BigDecimal = e.amount.to_string().parse().unwrap();
                                let multipool = event.log.address;

                                let price = match self
                                    .app_state
                                    .stats_cache
                                    .get(&multipool)
                                    .unwrap()
                                    .get_price(block.timestamp)
                                {
                                    Some(p) => p,
                                    None => {
                                        crate::price_fetcher::get_mps_prices(
                                            &[multipool],
                                            &self.app_state.provider,
                                            block.number,
                                        )
                                        .await?
                                        .0[0]
                                    }
                                };
                                let price: BigDecimal = price.to_string().parse().unwrap();
                                let price = price / (BigDecimal::from(1u128 << 96));

                                if !e.from.is_zero() {
                                    sqlx::query(
                                        "INSERT INTO actions_history(
                                            chain_id, 
                                            account, 
                                            multipool, 
                                            quantity,
                                            quote_quantity,
                                            transaction_hash,
                                            block_number,
                                            timestamp
                                        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8);",
                                    )
                                    .bind::<i64>(chain_id as i64)
                                    .bind::<[u8; 20]>(e.from.into())
                                    .bind::<[u8; 20]>(multipool.into())
                                    .bind::<BigDecimal>(-amount.clone())
                                    .bind::<BigDecimal>(-(amount.clone() * price.clone()))
                                    .bind::<[u8; 32]>(transaction.hash)
                                    .bind::<i64>(block.number as i64)
                                    .bind::<i64>(block.timestamp as i64)
                                    .execute(&mut **db_tx)
                                    .await?;
                                }
                                if !e.to.is_zero() {
                                    sqlx::query(
                                        "INSERT INTO actions_history(
                                            chain_id, 
                                            account, 
                                            multipool, 
                                            quantity,
                                            quote_quantity,
                                            transaction_hash,
                                            block_number,
                                            timestamp
                                        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8);",
                                    )
                                    .bind::<i64>(chain_id as i64)
                                    .bind::<[u8; 20]>(e.to.into())
                                    .bind::<[u8; 20]>(multipool.into())
                                    .bind::<BigDecimal>(amount.clone())
                                    .bind::<BigDecimal>(amount * price)
                                    .bind::<[u8; 32]>(transaction.hash)
                                    .bind::<i64>(block.number as i64)
                                    .bind::<i64>(block.timestamp as i64)
                                    .execute(&mut **db_tx)
                                    .await?;
                                }
                            }
                            MultipoolEvents::OwnershipTransferred(e) => {
                                sqlx::query(
                                    "
                                        UPDATE multipools 
                                        SET owner = $1
                                        WHERE multipool = $2;
                                    ",
                                )
                                .bind::<[u8; 20]>(e.newOwner.into())
                                .bind::<[u8; 20]>(event.log.address.into())
                                .execute(&mut **db_tx)
                                .await?;
                            }
                            MultipoolEvents::AssetChange(e) => {
                                if e.asset == event.log.address {
                                    sqlx::query(
                                        "
                                        UPDATE multipools 
                                        SET total_supply = $1::NUMERIC
                                        WHERE multipool = $2;
                                    ",
                                    )
                                    .bind::<String>(e.quantity.to_string())
                                    .bind::<[u8; 20]>(event.log.address.into())
                                    .execute(&mut **db_tx)
                                    .await?;
                                    self.app_state
                                        .stats_cache
                                        .get_mut(&e.asset)
                                        .unwrap()
                                        .insert_total_supply(e.quantity.to());
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
