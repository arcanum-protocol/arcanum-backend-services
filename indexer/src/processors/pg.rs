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
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use serde_json::Value;
use sqlx::{types::BigDecimal, Acquire, Postgres, Transaction};
use std::time::Duration;

fn pg_bytes(bytes: &[u8]) -> String {
    format!("\\x{}", bytes.encode_hex())
}

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
    pub rpc: P,
}

impl<P: Provider + Clone + 'static> Processor<Transaction<'static, Postgres>>
    for PgEventProcessor<P>
{
    async fn process(
        &mut self,
        logs: &[indexer1::alloy::rpc::types::Log],
        transaction: &mut Transaction<'static, Postgres>,
        _prev_saved_block: u64,
        _new_saved_block: u64,
        chain_id: u64,
    ) -> anyhow::Result<()> {
        let blocks = Blocks::parse_logs(logs, self.rpc.clone())
            .await
            .map_err(|_e| anyhow!("ParseLogsErrror"))?;

        let creations: MultipoolsCreation = blocks.0.as_slice().try_into()?;
        for creation in creations.0.into_iter() {
            sqlx::query(
                "INSERT INTO multipools(
                    chain_id,
                    multipool,
                    creator
                ) VALUES ($1, $2, $3);",
            )
            .bind::<i64>(chain_id.try_into()?)
            .bind::<&[u8]>(creation.multipool_address.as_slice())
            .bind::<&[u8]>(creation.address.as_slice())
            .execute(transaction.acquire().await?)
            .await?;
        }
        let creations: MultipoolsCreation = blocks.0.as_slice().try_into()?;

        for block in blocks.0.iter() {
            sqlx::query(
                "INSERT INTO blocks(
                    chain_id, 
                    block_number,
                    block
                    ) VALUES ($1,$2, $3);",
            )
            .bind::<i64>(chain_id.try_into()?)
            .bind::<BigDecimal>(block.number.into())
            .bind::<Value>(to_value(block)?)
            .execute(transaction.acquire().await?)
            .await?;

            let actions: Vec<TradingAction> = block
                .transactions
                .iter()
                .map(|txn| {
                    txn.events.iter().map(|event| {
                        let mut res = Vec::new();

                        if let Ok(parsed_log) = MultipoolEvents::decode_log(&event.log, false) {
                            match parsed_log.data {
                                MultipoolEvents::ShareTransfer(e) => {
                                    if e.to != Address::ZERO {
                                        res.push(TradingAction {
                                            account: pg_bytes(e.to.as_slice()),
                                            multipool: pg_bytes(event.log.address.as_slice()),
                                            chain_id: chain_id as i64,
                                            action_type: "receive".to_string(),
                                            quantity: e.amount.to_string(),
                                            quote_quantity: None,
                                            transaction_hash: pg_bytes(txn.hash.as_slice()),
                                            timestamp: block.timestamp as i64,
                                        });
                                    }
                                    if e.from != Address::ZERO {
                                        res.push(TradingAction {
                                            account: pg_bytes(e.from.as_slice()),
                                            multipool: pg_bytes(event.log.address.as_slice()),
                                            chain_id: chain_id as i64,
                                            action_type: "send".to_string(),
                                            quantity: e.amount.to_string(),
                                            quote_quantity: None,
                                            transaction_hash: pg_bytes(txn.hash.as_slice()),
                                            timestamp: block.timestamp as i64,
                                        });
                                    }
                                }
                                _ => (),
                            }
                        }
                        res
                    })
                })
                .flatten()
                .flatten()
                .collect::<Vec<_>>();
            while let Err(e) = sqlx::query("call insert_history($1::JSON);")
                .bind::<serde_json::Value>(serde_json::to_value(&actions).unwrap())
                .execute(transaction.acquire().await?)
                .await
            {
                println!("{actions:?}");
                println!("{e}");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            //TODO: add trade insert in case
            //
            //TODO: add multipool and multipool assets management
        }

        Ok(())
    }
}
