use alloy::{
    primitives::{address, Address},
    sol,
};

pub mod borsh_methods;
pub mod expiry;
pub mod messages;

pub const FACTORY_ADDRESS: Address = address!("1A9071F29731088650DbbB21a7bD7248a91d33cA");

sol!(
    #[allow(clippy::too_many_arguments)]
    #[allow(missing_docs)]
    #[derive(serde::Serialize)]
    #[sol(rpc, abi)]
    Multipool,
    "../../arcanum-contracts/out/Multipool.sol/Multipool.json"
);

sol!(
    #[allow(clippy::too_many_arguments)]
    #[allow(missing_docs)]
    #[sol(rpc)]
    MultipoolFactory,
    "../../arcanum-contracts/out/Factory.sol/MultipoolFactory.json"
);

sol!(
    #[allow(clippy::too_many_arguments)]
    #[allow(missing_docs)]
    #[sol(rpc)]
    Proxy,
    "../../arcanum-contracts/out/ERC1967Proxy.sol/ERC1967Proxy.json"
);

sol!(
    #[allow(clippy::too_many_arguments)]
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20,
    "../../arcanum-contracts/out/ERC20.sol/MockERC20.json"
);
