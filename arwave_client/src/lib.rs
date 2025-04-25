use crate::common::Tag;
use anyhow::Result;

pub mod common;
pub mod rpc;
pub mod wallet;
pub mod transaction;
pub mod utils;

pub struct CreateTransaction {
    format: i32,
    last_tx: String,
    owner: String,
    tags: Vec<Tag>,
    target: String,
    quantity: String,
    data: Vec<u8>,
    data_size: String,
    data_root: String,
    reward: String,
}
pub struct Arwave {}

impl Arwave {}
