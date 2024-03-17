use std::{str::FromStr, time::Duration};

use anyhow::Result;
use dashmap::DashMap;

use ethers::prelude::*;
use multipool_storage::{MultipoolStorage, MultipoolStorageHook, StorageEntry};

use crate::crypto::{self, SignedSharePrice};

#[derive(Default)]
pub struct CachedMultipoolData {
    cached_price: DashMap<Address, SignedSharePrice>,
}

impl CachedMultipoolData {
    pub fn get_signed_price(&self, etf_address: &Address) -> Option<SignedSharePrice> {
        self.cached_price.get(etf_address).as_deref().cloned()
    }

    pub async fn refresh<H: MultipoolStorageHook + 'static>(
        &self,
        storage: MultipoolStorage<H>,
        interval: u64,
        price_ttl: u64,
        chain_id: u128,
        key: String,
    ) -> Result<()> {
        loop {
            let pools = storage.pools().await;
            let signer = Wallet::from_str(&key).unwrap();
            for StorageEntry { multipool, address } in pools {
                if let Err(e) = multipool
                    .read()
                    .await
                    .multipool
                    .get_price(&address)
                    .map(|p| {
                        p.not_older_than(price_ttl).map(|price| {
                            self.cached_price
                                .insert(address, crypto::sign(address, price, chain_id, &signer))
                        })
                    })
                {
                    println!("{e:?}");
                }
            }
            tokio::time::sleep(Duration::from_millis(interval)).await;
        }
    }
}
