use alloy::{
    primitives::{Address, Log},
    sol_types::SolEventInterface,
};
use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use itertools::Itertools;
use multipool::Multipool;
use multipool_types::Multipool::MultipoolEvents;
use sled::{transaction::ConflictableTransactionResult, Transactional};
use tokio::task::JoinHandle;

use crate::hook::HookInitializer;

pub struct MultipoolStorage<HI: HookInitializer> {
    multipools: sled::Tree,
    block_number: sled::Tree,
    hooks: Vec<JoinHandle<Result<()>>>,
    hook_initializer: HI,
}

impl<HI: HookInitializer> MultipoolStorage<HI> {
    pub async fn init(db: sled::Db, mut hook_initializer: HI) -> Result<Self> {
        let multipools = db.open_tree(b"multipools")?;
        let block_number = db.open_tree(b"current_block")?;
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
            block_number,
            hooks,
            hook_initializer,
        })
    }

    pub async fn apply_events<I: IntoIterator<Item = Log>>(
        &mut self,
        logs: I,
        from_block: u64,
        to_block: u64,
    ) -> anyhow::Result<()> {
        let grouped_events = logs
            .into_iter()
            .chunk_by(|log| log.address)
            .into_iter()
            .map(|(address, events)| {
                (
                    address,
                    events
                        .map(|e| MultipoolEvents::decode_log(&e, true).unwrap().data)
                        .collect::<Vec<MultipoolEvents>>(),
                )
            })
            .collect::<Vec<_>>();
        let new_pools = (&self.multipools, &self.block_number)
            .transaction(|(multipools, blocks)| -> ConflictableTransactionResult<Vec<Address>, anyhow::Error> {
                let mut new_pools = Vec::new();
                if let Some(value) = blocks.get(b"current_block")? {
                    let value = u64::deserialize(&mut &value[..]).unwrap();
                    assert_eq!(value, from_block + 1);
                }
                for (address, events) in &grouped_events {
                    let mut mp = match multipools.get(address)? {
                        Some(mp) => Multipool::deserialize(&mut &mp[..]).unwrap(),
                        None => {
                            new_pools.push(*address);
                            Multipool::new(*address)
                        },
                    };

                    mp.apply_events(&events);

                    let mut w = Vec::new();
                    Multipool::serialize(&mp, &mut w).unwrap();
                    multipools.insert(address.as_slice(), w)?;
                }
                let mut serialized_to_block = Vec::new();
                to_block.serialize(&mut serialized_to_block).unwrap();
                blocks.insert(b"current_block", &to_block.to_be_bytes())?;
                Ok(new_pools)
            })
            .unwrap();
        self.block_number.flush()?;
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
