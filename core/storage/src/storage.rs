use alloy::{
    primitives::{aliases::U128, keccak256, Address},
    rpc::types::Log,
    sol_types::{SolEventInterface, SolValue},
};
use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use itertools::Itertools;
use multipool::Multipool;
use multipool_types::Multipool::MultipoolEvents;
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

fn parse_log(log: Log) -> Option<MultipoolEvents> {
    let log = alloy::primitives::Log {
        address: log.inner.address,
        data: log.inner.data,
    };
    MultipoolEvents::decode_log(&log, true).ok().map(|l| l.data)
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
                let mp = Multipool::deserialize(&mut &mp[..]).unwrap();
                mp
            };
            let handle = hook_initializer.initialize_hook(getter).await;
            hooks.push(handle);
        }

        Ok(Self {
            multipools,
            index_data,
            hooks,
            hook_initializer,
            factory_address,
        })
    }

    pub async fn apply_events<I: IntoIterator<Item = Log>>(
        &mut self,
        logs: I,
        from_block: u64,
        to_block: u64,
    ) -> anyhow::Result<()> {
        let value = self
            .index_data
            .get(b"current_block")?
            .map(|value| u64::deserialize(&mut &value[..]).unwrap())
            .unwrap_or(0);

        if from_block != value + 1 {
            return Err(anyhow!("data potentially skipped"));
        }

        let grouped_events = logs
            .into_iter()
            .chunk_by(|log| log.inner.address)
            .into_iter()
            .map(|(address, events)| {
                (
                    address,
                    events
                        .filter_map(|e| {
                            if value > e.block_number.expect("log should contain block number") {
                                None
                            } else {
                                parse_log(e)
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
                            let nonce = self
                                .index_data
                                .get(b"factory_nonce")?
                                .map(|value| multipool_types::borsh_methods::deserialize::u128(&mut &value[..]).unwrap())
                                .unwrap_or(U128::from(0));

                            // TODO: we need to check seed and check that derived addresses match
                            let mut bytes = Vec::new();
                            bytes.extend_from_slice(self.factory_address.abi_encode().as_slice());
                            bytes.extend_from_slice(&nonce.to_le_bytes::<16>());

                            let expected_address = Address::from_word(keccak256(bytes));
                            if *address != expected_address {
                                continue;
                            } else {
                                // IF all ok, we store stuff
                                // TODO check all endiness of storage
                                index_data.insert(b"factory_nonce", &(nonce + U128::from(1)).to_be_bytes::<16>())?;
                                new_pools.push(*address);
                                Multipool::new(*address)
                            }

                        },
                    };

                    mp.apply_events(&events);

                    let mut w = Vec::new();
                    Multipool::serialize(&mp, &mut w).unwrap();
                    multipools.insert(address.as_slice(), w)?;
                }
                let mut serialized_to_block = Vec::new();
                to_block.serialize(&mut serialized_to_block).unwrap();
                index_data.insert(b"current_block", &to_block.to_be_bytes())?;
                Ok(new_pools)
            })
            .unwrap();
        self.index_data.flush()?;
        self.multipools.flush()?;

        for mp_address in new_pools {
            let tree = self.multipools.clone();
            let getter = move || {
                let mp = tree.get(mp_address).unwrap().unwrap();
                let mp = Multipool::deserialize(&mut &mp[..]).unwrap();
                mp
            };
            let handle = self.hook_initializer.initialize_hook(getter).await;
            self.hooks.push(handle);
        }
        Ok(())
    }
}
