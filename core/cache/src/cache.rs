use std::{str::FromStr, time::Duration};

use alloy::{network::EthereumWallet, primitives::Address, signers::local::PrivateKeySigner};
use anyhow::Result;
use dashmap::DashMap;

use multipool_storage::storage::MultipoolStorage;

use multipool::Multipool;
use multipool_storage::hook::HookInitializer;

use crate::crypto::{self, SignedSharePrice};

#[derive(Default)]
pub struct CachedMultipoolData {
    cached_price: DashMap<Address, SignedSharePrice>,
    cached_pools: DashMap<Address, Multipool>,
}

impl CachedMultipoolData {
    pub fn get_signed_price(&self, etf_address: &Address) -> Option<SignedSharePrice> {
        self.cached_price.get(etf_address).as_deref().cloned()
    }

    pub fn get_pool(&self, etf_address: &Address) -> Option<Multipool> {
        self.cached_pools.get(etf_address).as_deref().cloned()
    }

    pub fn get_pools(&self) -> Vec<Multipool> {
        self.cached_pools
            .iter()
            .map(|r| r.value().to_owned())
            .collect()
    }

    pub async fn refresh<H: HookInitializer + 'static>(
        &self,
        storage: MultipoolStorage<H>,
        interval: u64,
        price_ttl: u64,
        chain_id: u128,
        key: String,
    ) -> Result<()> {
        todo!()
        // loop {
        //     let pools = storage.pools().await;
        //     let signer = PrivateKeySigner::from_str(&key)?;
        //     for StorageEntry { multipool, address } in pools {
        //         let mp = multipool.read().await.multipool.to_owned();
        //         if let Err(e) = mp.get_price(&address).map(|p| {
        //             p.not_older_than(price_ttl).map(|price| {
        //                 self.cached_price.insert(
        //                     address,
        //                     crypto::sign(address, price, chain_id, &signer).unwrap(),
        //                 )
        //             })
        //         }) {
        //             log::warn!(
        //                 "{}",
        //                 serde_json::to_string(&serde_json::json!({
        //                     "target": "storage-cache",
        //                     "error": format!("{e:?}"),
        //                     "address": address,
        //                     "message": "Failed to cache"
        //                 }))
        //                 .unwrap()
        //             );
        //         }
        //         self.cached_pools.insert(address, mp);
        //     }
        //     tokio::time::sleep(Duration::from_millis(interval)).await;
        // }
    }
}
