use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use multipool::Multipool;
use std::{cell::RefCell, rc::Rc, str::FromStr};
use wasm_bindgen::prelude::*;

use crate::{MultipoolWasmStorage, MultipoolWasmStorageInner};

#[wasm_bindgen]
impl MultipoolWasmStorage {
    #[wasm_bindgen(constructor)]
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
        let provider = Provider::new(Http::from_str("https://eth.llamarpc.com").unwrap());
        let inner = Rc::new(RefCell::new(MultipoolWasmStorageInner {
            multipool: Multipool::new(multipool_address.parse().unwrap()),
            assets,
            provider,
        }));
        MultipoolWasmStorage { inner }
    }

    #[wasm_bindgen]
    pub async fn get_price(&self) -> String {
        let mp = &self.inner.borrow().multipool;
        mp.get_price(&mp.contract_address())
            .unwrap()
            .any_age()
            .to_string()
    }
}
