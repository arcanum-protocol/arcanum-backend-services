use std::{cell::RefCell, rc::Rc};

pub mod adapters;
pub mod contracts;
pub mod read;
//pub mod trade_data;

use ethers::{
    prelude::*,
    providers::{Http, Provider},
};
use multipool::Multipool;
use wasm_bindgen::prelude::*;

pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub struct MultipoolWasmStorageInner {
    pub multipool: Multipool,
    pub assets: Vec<Address>,
    pub provider: Provider<Http>,
}

#[wasm_bindgen]
pub struct MultipoolWasmStorage {
    inner: Rc<RefCell<MultipoolWasmStorageInner>>,
}

//macro_rules! log {
//    ( $( $t:tt )* ) => {
//        web_sys::console::log_1(&format!( $( $t )* ).into());
//    }
//}
