use lazy_static::lazy_static;

use super::*;
pub mod errors;
pub mod multipool_builder;
pub mod read;
pub mod write;

lazy_static! {
    pub static ref ADDRESSES: [Address; 5] = [
        Address::from_slice(&1_u64.to_le_bytes()),
        Address::from_slice(&2_u64.to_le_bytes()),
        Address::from_slice(&3_u64.to_le_bytes()),
        Address::from_slice(&4_u64.to_le_bytes()),
        Address::from_slice(&5_u64.to_le_bytes()),
    ];
}
