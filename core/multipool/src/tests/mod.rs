use ethers::prelude::*;
use lazy_static::lazy_static;

use super::*;
pub mod errors;
pub mod expiry;
pub mod multipool_builder;
pub mod read;
pub mod write;

lazy_static! {
    pub static ref ADDRESSES: [Address; 5] = [
        H160::from_low_u64_le(1),
        H160::from_low_u64_le(2),
        H160::from_low_u64_le(3),
        H160::from_low_u64_le(4),
        H160::from_low_u64_le(5),
    ];
}
