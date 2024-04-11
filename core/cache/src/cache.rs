use std::{str::FromStr, time::Duration};

use anyhow::Result;
use dashmap::DashMap;

use ethers::prelude::*;
use multipool_storage::{MultipoolStorage, MultipoolStorageHook, StorageEntry};

use multipool::{expiry::StdTimeExtractor, Multipool};

use crate::crypto::{self, SignedSharePrice};

#[derive(Default)]
pub struct CachedMultipoolData {
    cached_price: DashMap<Address, SignedSharePrice>,
    cached_pools: DashMap<Address, Multipool<StdTimeExtractor>>,
}

impl CachedMultipoolData {
    pub fn get_signed_price(&self, etf_address: &Address) -> Option<SignedSharePrice> {
        self.cached_price.get(etf_address).as_deref().cloned()
    }

    pub fn get_pool(&self, etf_address: &Address) -> Option<Multipool<StdTimeExtractor>> {
        self.cached_pools.get(etf_address).as_deref().cloned()
    }

    pub fn get_pools(&self) -> Vec<Multipool<StdTimeExtractor>> {
        self.cached_pools
            .iter()
            .map(|r| r.value().to_owned())
            .collect()
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
                let mp = multipool.read().await.multipool.to_owned();
                if let Err(e) = mp.get_price(&address).map(|p| {
                    p.not_older_than(price_ttl).map(|price| {
                        self.cached_price
                            .insert(address, crypto::sign(address, price, chain_id, &signer))
                    })
                }) {
                    log::warn!(
                        "{}",
                        serde_json::to_string(&serde_json::json!({
                            "target": "storage-cache",
                            "error": format!("{e:?}"),
                            "address": address,
                            "message": "Failed to cache"
                        }))
                        .unwrap()
                    );
                }
                self.cached_pools.insert(address, mp);
            }
            tokio::time::sleep(Duration::from_millis(interval)).await;
        }
    }
}
