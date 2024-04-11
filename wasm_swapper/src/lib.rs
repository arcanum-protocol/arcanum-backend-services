use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub mod adapters;
pub mod contracts;
pub mod read;
//pub mod trade_data;

use ethers::{
    prelude::*,
    providers::{Http, Provider},
};
use multipool::{expiry::WasmTimeExtractor, Multipool};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub struct MultipoolWasmStorageInner {
    pub multipool: Multipool<WasmTimeExtractor>,
    pub assets: Option<Vec<Address>>,
    pub provider: Provider<Http>,
    pub assets_data: Option<AssetsStorage>,
}

#[wasm_bindgen]
pub struct MultipoolWasmStorage {
    inner: Rc<RefCell<MultipoolWasmStorageInner>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AssetsStorage {
    assets: HashMap<Address, Asset>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Asset {
    address: Address,
    symbol: String,
    name: String,
    decimals: u8,
    logo_url: Option<String>,
    twitter_url: Option<String>,
    description: Option<String>,
    website_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UniswapPool {
    asset_address: Address,
    pool_address: Address,
    base_is_asset0: bool,
    fee: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SiloPool {
    asset_address: Address,
    base_asset_address: Address,
    pool_address: Address,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AssetsResponse {
    assets: Vec<Asset>,
    uniswap_pools: Vec<UniswapPool>,
    silo_pools: Vec<SiloPool>,
}
