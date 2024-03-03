use ethers::prelude::*;

pub struct ObservationStorage {
    pub pool_address: Address,
    pub observations: Vec<(U256, U256)>,
}
