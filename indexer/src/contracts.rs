use alloy::{
    primitives::{Address, U256},
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

impl MultipoolSpawnedEvent {
    pub fn new_from_event(ev: MultipoolFactory::MultipoolSpawned) -> Self {
        Self {
            address: ev._0,
            number: ev.number,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TargetShareChangeEvent {
    pub asset: Address,
    pub new_target_share: U256,
    pub new_total_target_shares: U256,
}

impl TargetShareChangeEvent {
    pub fn new_from_event(ev: Multipool::TargetShareChange) -> Self {
        Self {
            asset: ev.asset,
            new_target_share: ev.newTargetShare,
            new_total_target_shares: ev.newTotalTargetShares,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct AssetChangeEvent {
    pub asset: Address,
    pub quantity: U256,
    pub collected_cashbacks: U256,
}

impl AssetChangeEvent {
    pub fn new_from_event(ev: Multipool::AssetChange) -> Self {
        Self {
            asset: ev.asset,
            quantity: ev.quantity,
            collected_cashbacks: U256::from(ev.collectedCashbacks),
        }
    }
}
