use alloy::primitives::Address;
use multipool::{expiry::TimeExtractor, Multipool};
use multipool_storage::MultipoolWithMeta;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone, Debug)]
pub struct MultipoolStorage {
    db: sled::Db,
}

impl MultipoolStorage {
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }

    pub fn get_multipools<T: TimeExtractor + Serialize + DeserializeOwned>(
        &self,
    ) -> anyhow::Result<Vec<Multipool<T>>> {
        let mut multipools = vec![];

        for item in self.db.iter() {
            let multipool: Multipool<T> = bincode::deserialize(&item?.1)?;
            multipools.push(multipool);
        }
        Ok(multipools)
    }

    pub fn get_multipool<T: TimeExtractor + Serialize + DeserializeOwned>(
        &self,
        address: Address,
    ) -> anyhow::Result<Option<Multipool<T>>> {
        let val = self.db.get(address.to_string())?;
        Ok(val.map(|x| bincode::deserialize(&x).unwrap()))
    }

    pub fn insert_multipools(&self, multipools: Vec<(Address, u64)>) -> anyhow::Result<()> {
        let mut batch = sled::Batch::default();

        for (address, start_block) in multipools {
            let multipool_with_meta = MultipoolWithMeta::new(address, start_block);
            batch.insert(
                address.to_string().as_bytes(),
                bincode::serialize(&multipool_with_meta)?,
            );
        }

        self.db.apply_batch(batch)?;

        Ok(())
    }

    pub fn update_multipool<F: Fn(&mut MultipoolWithMeta)>(
        &self,
        address: Address,
        update_fn: F,
    ) -> anyhow::Result<Option<MultipoolWithMeta>> {
        let prev_val = self
            .db
            .fetch_and_update(address.to_string(), move |old_mp| {
                if let None = old_mp {
                    return None;
                }
                let mut mp = bincode::deserialize(&old_mp.unwrap()).unwrap();
                update_fn(&mut mp);
                bincode::serialize(&mp).ok()
            })?;
        Ok(prev_val.map(|x| bincode::deserialize(&x).unwrap()))
    }

    pub fn exists(&self, address: Address) -> anyhow::Result<bool> {
        Ok(self.db.contains_key(address.to_string())?)
    }
}
