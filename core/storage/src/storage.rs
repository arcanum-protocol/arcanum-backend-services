use std::collections::BTreeMap;

use alloy::{
    primitives::{aliases::U128, keccak256, Address, U256},
    rpc::types::Log,
    sol_types::{SolEventInterface, SolValue},
};
use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use multipool::Multipool;
use multipool_types::{
    expiry::{EmptyTimeExtractor, MayBeExpired},
    messages::Block,
    Multipool::MultipoolEvents,
    MultipoolFactory::MultipoolFactoryEvents,
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

pub struct MultipoolsUpdates {
    from_block_number: u64,
    to_block_number: u64,
    updates: Vec<MultipoolUpdates>,
}

pub struct MultipoolUpdates {
    address: Address,
    logs: Vec<MultipoolEvents>,
}

pub struct MultipoolsCreation(pub Vec<MultipoolCreation>);

pub struct MultipoolCreation {
    address: Address,
    multipool_address: Address,
}

impl TryFrom<&[Block]> for MultipoolsCreation {
    type Error = anyhow::Error;

    fn try_from(value: &[Block]) -> Result<Self, Self::Error> {
        let mut multipools = Vec::new();

        for block in value {
            for transaction in block.transactions.iter() {
                for event in transaction.events.iter() {
                    let mp = MultipoolFactoryEvents::decode_log(&event.log, false)?;
                    if let MultipoolFactoryEvents::MultipoolCreated(multipool_address) = mp.data {
                        multipools.push(MultipoolCreation {
                            address: event.log.address,
                            multipool_address: multipool_address.multipoolAddress,
                        });
                    }
                }
            }
        }
        Ok(MultipoolsCreation(multipools))
    }
}

impl TryFrom<Vec<Block>> for MultipoolsUpdates {
    type Error = anyhow::Error;

    fn try_from(value: Vec<Block>) -> Result<Self, Self::Error> {
        let from_block_number = value
            .first()
            .map(|v| v.number)
            .ok_or(anyhow!("Empty batch"))?;
        let to_block_number = value
            .last()
            .map(|v| v.number)
            .ok_or(anyhow!("Empty batch"))?;

        let mut updates = BTreeMap::<Address, Vec<MultipoolEvents>>::new();

        for block in value {
            for transaction in block.transactions {
                for event in transaction.events {
                    let entry = updates.entry(event.log.address).or_default();
                    let parsed_log = MultipoolEvents::decode_log(&event.log, false)?;
                    entry.push(parsed_log.data);
                }
            }
        }
        Ok(MultipoolsUpdates {
            from_block_number,
            to_block_number,
            updates: updates
                .into_iter()
                .map(|(address, logs)| MultipoolUpdates { address, logs })
                .collect(),
        })
    }
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

    pub async fn create_multipools(&mut self, creations: MultipoolsCreation) -> anyhow::Result<()> {
        for creation in creations.0 {
            let multipool = Multipool::new(creation.address);

            let mut w = Vec::new();
            Multipool::serialize(&multipool, &mut w).unwrap();
            self.multipools
                .insert(creation.multipool_address.as_slice(), w)?;

            let tree = self.multipools.clone();
            let multipool_getter = move || {
                let mp = tree
                    .get(creation.multipool_address.as_slice())
                    .unwrap()
                    .unwrap();
                Multipool::deserialize(&mut &mp[..]).unwrap()
            };
            let mut handles = self
                .hook_initializer
                .initialize_hook(multipool_getter)
                .await;
            self.hooks.append(&mut handles);
        }
        self.multipools.flush()?;
        Ok(())
    }

    pub async fn apply_events(&mut self, updates: MultipoolsUpdates) -> anyhow::Result<()> {
        let current_block = self
            .index_data
            .get(b"current_block")?
            .map(|value| u64::deserialize(&mut &value[..]).unwrap());

        if current_block.is_some() && current_block.unwrap() >= updates.from_block_number {
            return Err(anyhow!("Data invalid"));
        }

        (&self.multipools, &self.index_data)
            .transaction(
                |(multipools, index_data)| -> ConflictableTransactionResult<(), anyhow::Error> {
                    for MultipoolUpdates { address, logs } in &updates.updates {
                        let mp = multipools
                            .get(address)?
                            .ok_or(anyhow!("Multipool not found"))
                            .unwrap(); // should retrun error
                        let mut mp = Multipool::deserialize(&mut &mp[..]).unwrap();

                        mp.apply_events(logs.as_slice());
                        let mut w = Vec::new();
                        Multipool::serialize(&mp, &mut w).unwrap();
                        multipools.insert(address.as_slice(), w)?;
                    }
                    let mut new_current_block = Vec::new();
                    updates
                        .to_block_number
                        .serialize(&mut new_current_block)
                        .unwrap();
                    index_data.insert(b"current_block", new_current_block)?;
                    Ok(())
                },
            )
            .unwrap();
        self.index_data.flush()?;
        self.multipools.flush()?;
        Ok(())
    }

    pub async fn apply_prices(
        &mut self,
        address: Address,
        prices: Vec<(Address, MayBeExpired<U256, EmptyTimeExtractor>)>,
    ) -> Result<()> {
        self.multipools
            .transaction(
                |multipools| -> ConflictableTransactionResult<(), anyhow::Error> {
                    if let Some(mp) = multipools.get(address)? {
                        let mut mp = Multipool::deserialize(&mut &mp[..]).unwrap();
                        mp.update_prices(&prices);
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
