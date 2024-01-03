pub mod chain;
//pub mod chain_adapter;

use std::collections::BTreeMap;
use std::ops::{Shl, Shr};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::providers::Provider;
use ethers::providers::{Http, Middleware};
use ethers::signers::Wallet;
use ethers::types::Address;
use ethers::types::U64;
use futures::future::join_all;
use futures::Future;
use primitive_types::U256;
use serde::{Deserialize, Serialize};
use tokio::spawn;
use tokio::time::sleep;

use crate::config::{BotConfig, MultipoolConfig};
use crate::crypto::{sign, SignedSharePrice};

use self::chain::MultipoolContractInterface;
use self::chain::QuantityUpdate;

pub type MultipoolId = String;
pub type Price = U256;
pub type BlockNumber = U64;
pub type Quantity = U256;

#[derive(Clone, Debug, Default)]
pub struct MultipoolStorage {
    state: Arc<DashMap<MultipoolId, Multipool>>,
}

impl MultipoolStorage {
    pub fn from_config(config: BotConfig) -> Self {
        let state = config
            .multipools
            .into_iter()
            .map(|(k, v)| (k, Multipool::from_config(v)))
            .collect();
        Self {
            state: Arc::new(state),
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct MayBeExpired<V> {
    inner: Option<(V, u64)>,
}

impl<V> MayBeExpired<V> {
    pub fn not_older_than(self, interval: u64) -> Option<V> {
        self.inner
            .map(|(v, timestamp)| {
                let current_timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Shold be always after epoch start")
                    .as_secs();
                if current_timestamp <= timestamp + interval {
                    Some(v)
                } else {
                    None
                }
            })
            .flatten()
    }
}

impl<V> From<V> for MayBeExpired<V> {
    fn from(value: V) -> Self {
        Self {
            inner: Some((
                value,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Shold be always after epoch start")
                    .as_secs(),
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct Multipool {
    fetch_params: MultipoolFetchParams,
    contract_address: Address,
    assets: BTreeMap<Address, MultipoolAsset>,
    total_supply: MayBeExpired<Quantity>,
    provider: Arc<Provider<Http>>,
    last_observed_block: BlockNumber,
    chain_id: u128,
}

impl Multipool {
    fn from_config(config: MultipoolConfig) -> Self {
        Self {
            fetch_params: config.fetch_params,
            contract_address: config.contract_address,
            assets: config
                .assets
                .into_iter()
                .map(|a| {
                    (
                        a,
                        MultipoolAsset {
                            address: a,
                            price: Default::default(),
                            quantity: Default::default(),
                        },
                    )
                })
                .collect(),
            total_supply: Default::default(),
            provider: Arc::new(
                Provider::<Http>::try_from(config.provider_url)
                    .expect("Provider url should be valid"),
            ),
            last_observed_block: U64::zero(),
            chain_id: config.chain_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipoolFetchParams {
    block_limit: BlockNumber,
    quantity_fetch_interval: u64,
    price_fetch_interval: u64,
    price_fetch_chunk: usize,
}

impl Multipool {
    fn contract(&self) -> MultipoolContractInterface {
        MultipoolContractInterface::new(self.contract_address, self.provider.clone())
    }

    fn get_price(&self, poison_time: u64) -> Option<Price> {
        let q = U256::from(96);
        let cap = self
            .assets
            .iter()
            .map(|(_, a)| -> Option<U256> {
                let quantity = a.quantity.to_owned().not_older_than(poison_time)?;
                let price = a.price.to_owned().not_older_than(poison_time)?;
                quantity.checked_mul(price).map(|mul| mul.shr(q))
            })
            .collect::<Option<Vec<U256>>>()?
            .into_iter()
            .reduce(|sum, el| sum + el)?;
        let total_supply = self.total_supply.to_owned().not_older_than(poison_time)?;
        cap.shl(q).checked_div(total_supply)
    }
}

#[derive(Debug, Clone)]
struct MultipoolAsset {
    address: Address,
    quantity: MayBeExpired<Quantity>,
    price: MayBeExpired<Price>,
}

impl MultipoolStorage {
    pub fn get_signed_price(
        &self,
        id: &MultipoolId,
        signer: &Wallet<SigningKey>,
        poison_time: u64,
    ) -> Option<SignedSharePrice> {
        let mp = self
            .state
            .get(id)
            .expect("Multipool should present")
            .clone();
        mp.get_price(poison_time)
            .map(|price| sign(mp.contract_address, price, mp.chain_id, signer))
    }

    pub fn get_prices(&self, poison_time: u64) -> Vec<(MultipoolId, Option<Price>)> {
        self.state
            .iter()
            .map(|e| (e.key().to_owned(), e.value().clone().get_price(poison_time)))
            .collect()
    }

    fn fetch_price(&self, id: MultipoolId) -> impl Future<Output = ()> {
        let mp = self.state.get(&id).expect("Multipool should present");
        let assets = mp.assets.clone();
        let params = mp.fetch_params.clone();
        let contract = mp.contract();
        let state = self.state.clone();
        async move {
            for chunk in assets
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .chunks(params.price_fetch_chunk)
            {
                let prices = join_all(
                    chunk
                        .into_iter()
                        .map(|asset| contract.get_asset_price(asset.address)),
                )
                .await;
                {
                    let mut mp = state.get_mut(&id).expect("Multipool should present");
                    let assets = &mut mp.assets;
                    for (asset, price) in chunk.into_iter().zip(prices.into_iter().map(|val| {
                        val.unwrap_or_else(|error| {
                            println!("Price fetch should be successful: {:#?}", error);
                            std::process::exit(0x0200);
                        })
                    })) {
                        assets
                            .entry(asset.address)
                            .and_modify(|e| e.price = price.into());
                    }
                }
            }
        }
    }

    fn fetch_quantity(&self, id: MultipoolId) -> impl Future<Output = ()> {
        let mp = self.state.get(&id).expect("Multipool should present");
        let contract_address = mp.contract_address;
        let params = mp.fetch_params.clone();
        let contract = mp.contract();
        let state = self.state.clone();
        let provider = mp.provider.clone();
        let assets = mp.assets.clone();
        drop(mp);
        async move {
            {
                let current_block = provider.get_block_number().await.unwrap_or_else(|error| {
                    println!("Should correctly fetch block number: {:#?}", error);
                    std::process::exit(0x0300);
                });
                let mut mp = state.get_mut(&id).expect("Multipool should present");
                mp.last_observed_block = current_block;
                drop(mp);
            }
            {
                let total_supply = contract.get_total_supply().await.unwrap_or_else(|error| {
                    println!("Should correctly fetch total suppply: {:#?}", error);
                    std::process::exit(0x0400);
                });
                let mut mp = state.get_mut(&id).expect("Multipool should present");
                mp.total_supply = total_supply.into();
                drop(mp);
            }
            for chunk in assets
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .chunks(params.price_fetch_chunk)
            {
                let quantities = join_all(
                    chunk
                        .into_iter()
                        .map(|asset| contract.get_asset_quantity(asset.address)),
                )
                .await;
                {
                    let mut mp = state.get_mut(&id).expect("Multipool should present");
                    let assets = &mut mp.assets;
                    for (asset, quantity) in
                        chunk.into_iter().zip(quantities.into_iter().map(|val| {
                            val.unwrap_or_else(|error| {
                                println!("Quantity fetch should be successful: {:#?}", error);
                                std::process::exit(0x0100);
                            })
                        }))
                    {
                        assets
                            .entry(asset.address)
                            .and_modify(|e| e.quantity = quantity.into());
                    }
                    drop(mp);
                }
            }
            loop {
                let mp = state.get(&id).expect("Multipool should present");
                let last_block = mp.last_observed_block;
                drop(mp);
                let updates = QuantityUpdate::get_event_updates(
                    contract_address,
                    last_block,
                    params.block_limit,
                    provider.clone(),
                )
                .await
                .unwrap_or_else(|error| {
                    println!("Should successfully fetch events: {:#?}", error);
                    std::process::exit(0x0500);
                });
                let mut mp = state.get_mut(&id).expect("Multipool should present");
                for update in updates.into_iter() {
                    if update.address == mp.contract_address {
                        mp.total_supply = update.quantity.into();
                    } else if update.quantity.is_zero() {
                        mp.assets.remove(&update.address);
                    } else {
                        mp.assets
                            .entry(update.address)
                            .and_modify(|val| val.quantity = update.quantity.into())
                            .or_insert(MultipoolAsset {
                                address: update.address,
                                quantity: update.quantity.into(),
                                price: Default::default(),
                            });
                    }
                }
                drop(mp);
                sleep(Duration::from_millis(params.quantity_fetch_interval)).await;
            }
        }
    }

    pub fn gen_fetching_future(&self) -> impl Future<Output = ()> {
        let this = self.clone();
        async move {
            for entries in this.state.iter() {
                let this = this.clone();
                let key = entries.key().clone();
                println!("ID: {}", key);
                let params = entries.value().fetch_params.clone();
                {
                    let this = this.clone();
                    let key = entries.key().clone();
                    spawn(async move {
                        loop {
                            this.fetch_price(key.to_owned()).await;
                            sleep(Duration::from_millis(params.price_fetch_interval)).await
                        }
                    });
                }
                spawn(this.fetch_quantity(key.to_owned()));
            }
        }
    }
}
