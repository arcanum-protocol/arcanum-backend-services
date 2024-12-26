use alloy::{
    primitives::{Address, U256},
    signers::k256::pkcs8::der,
    sol,
};
use serde::Serialize;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Multipool,
    "src/abi/multipool.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    MultipoolFactory,
    "src/abi/multipool_factory.json"
);

#[derive(Serialize, Clone)]
pub struct MultipoolSpawnedEvent {
    pub address: Address,
    pub number: U256,
}

#[derive(Serialize, Clone)]
pub struct TargetShareChangeEvent {
    pub asset: Address,
    pub new_target_share: U256,
    pub new_total_target_shares: U256,
}

#[derive(Serialize, Clone)]
pub struct AssetChangeEvent {
    pub asset: Address,
    pub quantity: U256,
    pub collected_cashbacks: U256,
}
