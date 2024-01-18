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
use ethers::types::U64;
use ethers::types::{Address, I256};
use futures::future::join_all;
use futures::Future;
use log::info;
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
pub type Share = U256;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipoolFetchParams {
    block_limit: BlockNumber,
    quantity_fetch_interval: u64,
    price_fetch_interval: u64,
    price_fetch_chunk: usize,
}

#[derive(Debug, Clone)]
pub struct Multipool {
    pub fetch_params: MultipoolFetchParams,
    pub contract_address: Address,
    pub assets: BTreeMap<Address, MultipoolAsset>,
    pub total_supply: MayBeExpired<Quantity>,
    pub total_shares: MayBeExpired<Share>,
    pub provider: Arc<Provider<Http>>,
    pub last_observed_block: BlockNumber,
    pub chain_id: u128,
}

#[derive(Debug, Clone)]
pub struct MultipoolAsset {
    pub address: Address,
    pub quantity: MayBeExpired<Quantity>,
    pub price: MayBeExpired<Price>,
    pub share: MayBeExpired<Share>,
    pub cashback: MayBeExpired<Quantity>,
}

#[derive(Debug, Clone)]
pub struct BalancingData {
    pub deviation: f64,
    pub quantity_to_balance: I256,
    pub quantity_to_upper_bound: I256,
    pub quantity_to_lower_bound: I256,
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
                            share: Default::default(),
                            quantity: Default::default(),
                            cashback: Default::default(),
                        },
                    )
                })
                .collect(),
            total_supply: Default::default(),
            total_shares: Default::default(),
            provider: Arc::new(
                Provider::<Http>::try_from(config.provider_url)
                    .expect("Provider url should be valid"),
            ),
            last_observed_block: U64::zero(),
            chain_id: config.chain_id,
        }
    }
}

impl Multipool {
    fn contract(&self) -> MultipoolContractInterface {
        MultipoolContractInterface::new(self.contract_address, self.provider.clone())
    }

    pub fn get_quantities_to_balance(
        &self,
        share_price: U256,
        poison_time: u64,
    ) -> Option<Vec<(Address, BalancingData)>> {
        self.assets
            .clone()
            .into_iter()
            .map(
                |(asset_address, asset)| -> Option<(Address, BalancingData)> {
                    let quantity = asset.quantity.not_older_than(poison_time)?;
                    let price = asset.price.not_older_than(poison_time)?;
                    let share = asset.share.not_older_than(poison_time)?;
                    let total_shares = self.total_shares.clone().not_older_than(poison_time)?;
                    let total_supply = self.total_supply.clone().not_older_than(poison_time)?;

                    let usd_cap = total_supply * share_price;

                    let current_share = price * (quantity << 96) / usd_cap;

                    let target_share = share.shl(96).checked_div(total_shares).unwrap();

                    let pegged_quantity = share * usd_cap / price / total_shares;

                    let deviation_limit = U256::from(644245094) / 2;

                    let share_bound = (deviation_limit * total_shares) >> 32;
                    let upper = I256::from_raw(
                        (share + share_bound).min(total_shares) * usd_cap / price / total_shares,
                    ) - I256::from_raw(quantity);
                    let lower = I256::from_raw(
                        (share.checked_div(share_bound).unwrap_or(U256::zero())) * usd_cap
                            / price
                            / total_shares,
                    ) - I256::from_raw(quantity);

                    let pegged_quantity_modified =
                        I256::from_raw(pegged_quantity) - I256::from_raw(quantity);

                    Some((
                        asset_address,
                        BalancingData {
                            deviation: (I256::from_raw(current_share)
                                - I256::from_raw(target_share))
                            .as_i128() as f64
                                / 2f64.powf(96.0)
                                * 100f64,
                            quantity_to_balance: pegged_quantity_modified,
                            quantity_to_lower_bound: lower,
                            quantity_to_upper_bound: upper,
                        },
                    ))
                },
            )
            .collect()
    }

    fn get_price(&self, poison_time: u64) -> Option<Price> {
        let q = U256::from(96);
        let cap = self
            .assets
            .iter()
            .map(|(_, a)| -> Option<U256> {
                let quantity = match a.quantity.to_owned().not_older_than(poison_time) {
                    Some(v) => v,
                    None => {
                        log::error!("outdated quantity {}", a.address);
                        None?
                    }
                };
                let price = match a.price.to_owned().not_older_than(poison_time) {
                    Some(v) => v,
                    None => {
                        log::error!("outdated price {}", a.address);
                        None?
                    }
                };
                quantity.checked_mul(price).map(|mul| mul.shr(q))
            })
            .collect::<Option<Vec<U256>>>()?
            .into_iter()
            .reduce(|sum, el| sum + el)?;
        let total_supply = match self.total_supply.to_owned().not_older_than(poison_time) {
            Some(v) => v,
            None => {
                log::error!("outdated total supply");
                None?
            }
        };
        cap.shl(q).checked_div(total_supply)
    }
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

    pub fn get_multipools_data(&self) -> Vec<(MultipoolId, Multipool)> {
        self.state
            .iter()
            .map(|v| (v.key().clone(), v.value().clone()))
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
            {
                let total_shares = contract.get_total_shares().await.unwrap_or_else(|error| {
                    println!("Should correctly fetch total suppply: {:#?}", error);
                    std::process::exit(0x0400);
                });
                let mut mp = state.get_mut(&id).expect("Multipool should present");
                mp.total_shares = total_shares.into();
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
                        .map(|asset| contract.get_asset(asset.address)),
                )
                .await;
                {
                    let mut mp = state.get_mut(&id).expect("Multipool should present");
                    let assets = &mut mp.assets;
                    for (asset, asset_data) in
                        chunk.into_iter().zip(quantities.into_iter().map(|val| {
                            val.unwrap_or_else(|error| {
                                println!("Quantity fetch should be successful: {:#?}", error);
                                std::process::exit(0x0100);
                            })
                        }))
                    {
                        assets.entry(asset.address).and_modify(|e| {
                            e.quantity = asset_data.quantity.into();
                            e.share = asset_data.share.into();
                            e.cashback = asset_data.cashback.into();
                        });
                    }
                    drop(mp);
                }
            }
            loop {
                println!("loop quantity {}", id);
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
                                share: Default::default(),
                                price: Default::default(),
                                cashback: Default::default(),
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
                            println!("loop price {}", key);
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
