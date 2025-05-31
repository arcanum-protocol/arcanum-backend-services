use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use alloy::{
    primitives::{Address, U256},
    providers::{Provider, MULTICALL3_ADDRESS},
};
use dashmap::DashMap;
use k256::ecdsa::{Signature, SigningKey};
use multipool_types::{Multicall, Multipool};
use serde::Serialize;

pub struct PriceCache<P: Provider> {
    cache: DashMap<(Address, u64), CachedPrice>,
    provider: P,
    chain_id: u64,
    signer: Arc<SigningKey>,
}

//#[derive(Serialize)]
pub struct CachedPrice {
    timestamp: u64,
    block: u64,
    value: U256,
    signature: Signature,
}

//impl<P: Provider> PriceCache<P> {
//    pub async fn get_or_fetch(
//        &self,
//        address: Address,
//        block: u64,
//    ) -> anyhow::Result<CachedPrice> {
//        let cached_value = self.cache.get((&address, block));
//        if let Some(price) = cached_value {
//            let current_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
//            if current_ts - price.timestamp > expiration {
//
//            }
//        }
//        match cached_value {
//            Some(price) if price.timestamp
//            _ =>
//        }
//        let (price, timestamp) = get_price(self.provider, address, block).await?;
//    }
//}

pub async fn get_price<P: Provider>(
    provider: &P,
    address: Address,
    block: u64,
) -> anyhow::Result<(U256, u64)> {
    let (value, ts) = alloy::providers::MulticallBuilder::new(provider)
        .block(block.into())
        .add(Multipool::new(address, provider).getSharePricePart(U256::MAX, U256::ZERO))
        .get_current_block_timestamp()
        .aggregate3()
        .await?;

    Ok((value?, ts?.to()))
}
