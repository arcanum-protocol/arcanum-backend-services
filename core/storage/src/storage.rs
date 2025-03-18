use std::collections::HashMap;

use alloy::{
    primitives::{aliases::U128, keccak256, Address, U256},
    rpc::types::Log,
    sol_types::{SolEventInterface, SolValue},
};
use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use itertools::Itertools;
use multipool::{
    expiry::{EmptyTimeExtractor, MayBeExpired},
    Multipool,
};
use multipool_types::{
    kafka::{ChainBlock, ChainEvent},
    Multipool::MultipoolEvents,
};
use sled::{transaction::ConflictableTransactionResult, Transactional};
use tokio::task::JoinHandle;

use crate::hook::HookInitializer;

pub struct MultipoolStorage<HI: HookInitializer> {
    multipools: sled::Tree,
    index_data: sled::Tree,
    factory_address: Address,
    hooks: Vec<JoinHandle<Result<()>>>,
    hook_initializer: HI,
}

pub fn parse_log(log: Log) -> Option<MultipoolEvents> {
    let log = alloy::primitives::Log {
        address: log.inner.address,
        data: log.inner.data,
    };
    MultipoolEvents::decode_log(&log, false)
        .ok()
        .map(|l| l.data)
}

impl<HI: HookInitializer> MultipoolStorage<HI> {
    //TODO: store factory address in db
    pub async fn init(
        db: sled::Db,
        mut hook_initializer: HI,
        factory_address: Address,
    ) -> Result<Self> {
        let multipools = db.open_tree(b"multipools")?;
        let index_data = db.open_tree(b"index_data")?;
        let mut hooks = Vec::new();

        for val in multipools.into_iter() {
            let (address, _) = val?;
            let tree = multipools.clone();
            let getter = move || {
                let mp = tree.get(&address).unwrap().unwrap();
                Multipool::deserialize(&mut &mp[..]).unwrap()
            };
            let handles = hook_initializer.initialize_hook(getter).await;
            hooks.extend(handles);
        }

        Ok(Self {
            multipools,
            index_data,
            hooks,
            hook_initializer,
            factory_address,
        })
    }

    pub fn derive_multipool_address(
        factory_address: Address,
        factory_salt_nonce: U128,
        multipool_contract_bytecode: Vec<u8>,
    ) -> Address {
        let mut address_bytes: Vec<u8> = vec![0xff];
        address_bytes.extend_from_slice(factory_address.as_slice());
        address_bytes.extend_from_slice(&U256::from(factory_salt_nonce).abi_encode());
        address_bytes.extend_from_slice(keccak256(multipool_contract_bytecode).as_slice());

        Address::from_word(keccak256(address_bytes))
    }

    pub fn get_last_seen_block(&self) -> anyhow::Result<Option<u64>> {
        Ok(self
            .index_data
            .get(b"current_block")?
            .map(|value| u64::deserialize(&mut &value[..]).map(Into::into))
            .transpose()?)
    }

    pub async fn apply_events(&mut self, blocks: Vec<ChainBlock>) -> anyhow::Result<()> {
        let current_block = self
            .index_data
            .get(b"current_block")?
            .map(|value| u64::deserialize(&mut &value[..]).unwrap());

        let next_block = blocks.iter().map(|log| log.block_number).min();

        if current_block.is_some() && current_block != next_block.map(|v| v - 1) {
            return Err(anyhow!("data potentially skipped"));
        }

        let to_block = match blocks.iter().map(|log| log.block_number).max() {
            Some(b) => b,
            None => return Ok(()),
        };

        let logs = blocks
            .into_iter()
            .flat_map(|v|v.events)
            .collect::<Vec<ChainEvent>>();
        let grouped_events = logs
            .into_iter()
            .chunk_by(|log| log.row_event.inner.address)
            .into_iter()
            .map(|(address, events)| {
                (
                    address,
                    events
                        .filter_map(|e| {
                            if current_block.is_some()
                                && current_block.unwrap()
                                    > e.row_event
                                        .block_number
                                        .expect("log should contain block number")
                            {
                                None
                            } else {
                                parse_log(e.row_event)
                            }
                        })
                        .collect::<Vec<MultipoolEvents>>(),
                )
            })
            .collect::<Vec<_>>();

        let new_pools = (&self.multipools, &self.index_data)
            .transaction(|(multipools, index_data)| -> ConflictableTransactionResult<Vec<Address>, anyhow::Error> {
                let mut new_pools = Vec::new();

                for (address, events) in &grouped_events {
                    let mut mp = match multipools.get(address)? {
                        Some(mp) => Multipool::deserialize(&mut &mp[..]).unwrap(),
                        None => {
                            let nonce = index_data
                                .get(b"factory_nonce")?
                                .map(|value| u64::deserialize(&mut &value[..]).unwrap())
                                .unwrap_or(1);

                            let expected_address = self.factory_address.create(nonce);
                            if *address != expected_address {
                                continue;
                            } else {
                                let mut serialized_nonce = Vec::new();
                                u64::serialize(&(nonce + 1), &mut serialized_nonce).unwrap();

                                index_data.insert(b"factory_nonce", serialized_nonce)?;
                                new_pools.push(*address);
                                Multipool::new(*address)
                            }

                        },
                    };

                    mp.apply_events(events);
                    let mut w = Vec::new();
                    Multipool::serialize(&mp, &mut w).unwrap();
                    multipools.insert(address.as_slice(), w)?;
                }
                let mut current_block = Vec::new();
                to_block.serialize(&mut current_block).unwrap();
                index_data.insert(b"current_block", current_block)?;
                Ok(new_pools)
            })
            .unwrap();
        self.index_data.flush()?;
        self.multipools.flush()?;

        for mp_address in new_pools {
            let tree = self.multipools.clone();
            let multipool_getter = move || {
                let mp = tree.get(mp_address.as_slice()).unwrap().unwrap();
                Multipool::deserialize(&mut &mp[..]).unwrap()
            };
            let mut handles = self
                .hook_initializer
                .initialize_hook(multipool_getter)
                .await;
            self.hooks.append(&mut handles);
        }
        Ok(())
    }

    pub async fn apply_prices(
        &mut self,
        address: Address,
        prices: HashMap<Address, MayBeExpired<U256, EmptyTimeExtractor>>,
    ) -> Result<()> {
        self.multipools
            .transaction(
                |multipools| -> ConflictableTransactionResult<(), anyhow::Error> {
                    if let Some(mp) = multipools.get(address)? {
                        let mut mp = Multipool::deserialize(&mut &mp[..]).unwrap();
                        mp.update_maybe_prices(&prices);
                        let mut w = Vec::new();
                        Multipool::serialize(&mp, &mut w).unwrap();
                        multipools.insert(address.as_slice(), w)?;
                    }
                    Ok(())
                },
            )
            .unwrap();
        self.multipools.flush()?;
        Ok(())
    }
}
