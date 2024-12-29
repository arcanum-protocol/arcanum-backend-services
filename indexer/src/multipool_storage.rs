use alloy::primitives::Address;
use multipool::{expiry::TimeExtractor, Multipool};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub struct MultipoolStorage {
    db: sled::Db,
}

impl MultipoolStorage {
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }

    pub fn get_multipool<T: TimeExtractor + Serialize + DeserializeOwned>(
        &self,
        address: Address,
    ) -> anyhow::Result<Option<Multipool<T>>> {
        let val = self.db.get(address.to_string())?;
        Ok(val.map(|x| bincode::deserialize(&x).unwrap()))
    }

    pub fn insert_multipool<T: TimeExtractor + Serialize>(
        &self,
        address: Address,
        multipool: Multipool<T>,
    ) -> anyhow::Result<()> {
        self.db
            .insert(address.to_string(), bincode::serialize(&multipool)?)?;
        Ok(())
    }

    pub fn update_multipool<
        T: TimeExtractor + Serialize + DeserializeOwned,
        F: Fn(Multipool<T>) -> Multipool<T>,
    >(
        &self,
        address: Address,
        update_fn: F,
    ) -> anyhow::Result<Option<Multipool<T>>> {
        let prev_val = self
            .db
            .fetch_and_update(address.to_string(), move |old_mp| {
                if let None = old_mp {
                    return None;
                }
                let new_mp = update_fn(bincode::deserialize(&old_mp.unwrap()).unwrap());
                bincode::serialize(&new_mp).ok()
            })?;
        Ok(prev_val.map(|x| bincode::deserialize(&x).unwrap()))
    }
}
