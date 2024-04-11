use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use multipool::Multipool;
use std::{cell::RefCell, rc::Rc, str::FromStr};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsObject;

use crate::{
    Asset, AssetsResponse, AssetsStorage, MultipoolWasmStorage, MultipoolWasmStorageInner,
};

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct MultipoolAsset {
    pub price: String,
    pub target_share: String,
    pub current_share: String,
    pub deviation: String,
    pub cashbacks: String,
    pub quantity: String,
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub logo_url: Option<String>,
    pub twitter_url: Option<String>,
    pub description: Option<String>,
    pub website_url: Option<String>,
}

#[wasm_bindgen]
impl MultipoolWasmStorage {
    #[wasm_bindgen]
    pub async fn new(multipool_address: String) -> Self {
        let assets: Vec<Address> = reqwest::get(format!(
            "https://api.arcanum.to/oracle/v1/asset_list?multipool_address={}",
            multipool_address
        ))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
        let assets_data: AssetsResponse =
            reqwest::get("http://127.0.0.1:8080/assets?chain_id=42161")
                .await
                .expect("Failed to request")
                .json()
                .await
                .expect("Failed to parse");
        let provider = Provider::new(
            Http::from_str("https://arb-mainnet.g.alchemy.com/v2/MERXmvJOqhiBs4LYV_rOFMueneDC3Sq_")
                .unwrap(),
        );
        let inner = Rc::new(RefCell::new(MultipoolWasmStorageInner {
            multipool: Multipool::new(multipool_address.parse().unwrap()),
            assets: Some(assets),
            provider,
            assets_data: Some(AssetsStorage {
                assets: assets_data
                    .assets
                    .into_iter()
                    .map(|v| (v.address, v))
                    .collect(),
            }),
        }));
        MultipoolWasmStorage { inner }
    }

    #[wasm_bindgen]
    pub fn get_assets(&self) -> Result<Vec<MultipoolAsset>, JsValue> {
        let storage = &self.inner.borrow();
        let meta = storage
            .assets_data
            .as_ref()
            .ok_or(JsValue::from_str("assets missing"))?;
        storage
            .multipool
            .assets
            .iter()
            .map(|asset| -> Result<MultipoolAsset, JsValue> {
                let meta = meta.assets.get(&asset.address).ok_or("meta missing")?;
                Ok(MultipoolAsset {
                    price: asset
                        .price
                        .clone()
                        .ok_or("price missing")?
                        .any_age()
                        .to_string(),
                    target_share: asset
                        .share
                        .clone()
                        .ok_or("price missing")?
                        .any_age()
                        .to_string(),
                    cashbacks: asset
                        .quantity_slot
                        .clone()
                        .ok_or("price missing")?
                        .any_age()
                        .cashback
                        .to_string(),
                    current_share: "0".into(),
                    deviation: "0".into(),
                    quantity: "0".into(),
                    address: meta.address.to_string(),
                    symbol: meta.symbol.clone(),
                    name: meta.name.clone(),
                    decimals: meta.decimals.clone(),
                    logo_url: meta.logo_url.clone(),
                    twitter_url: meta.twitter_url.clone(),
                    description: meta.description.clone(),
                    website_url: meta.website_url.clone(),
                })
            })
            .collect()
    }

    #[wasm_bindgen]
    pub fn get_price(&self) -> String {
        let mp = &self.inner.borrow().multipool;
        match mp.get_price(&mp.contract_address()) {
            Ok(p) => p.any_age().to_string(),
            Err(e) => format!("{e:?}"),
        }
    }
}
