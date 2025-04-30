use alloy::{
    primitives::{address, Address},
    sol,
};

// pub const TRADER_ADDRESS: Address = address!("955aDe421294B9D9C49b09bd64a92a4138EA6C56");
// pub const TRADER_ADDRESS: Address = address!("1991E54D4d503086a9De4ff272c316f0ed4AA263");
pub const TRADER_ADDRESS: Address = address!("F69ae94063f4671Ea4e4b9f8c97eb1aAC1731cb8");
pub const CASHBACK_VAULT: Address = address!("B9cb365F599885F6D97106918bbd406FE09b8590");

// pub const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
pub const WETH_ADDRESS: Address = address!("760AfE86e5de5fa0Ee542fc7B7B713e1c5425701");

// pub const QUOTERV2_ADDRESS: Address = address!("61fFE014bA17989E743c5F6cB21bF9697530B21e");
pub const QUOTERV2_ADDRESS: Address = address!("1b4E313fEF15630AF3e6F2dE550Dbf4cC9D3081d"); // v1??
// pub const QUOTERV2_ADDRESS: Address = address!("1BA215c17565dE7b0Cb7eCaB971bcF540c24a862"); // v2??

pub const MULTICALL_ADDRESS: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");

pub const SILO_LENS: Address = address!("BDb843c7a7e48Dc543424474d7Aa63b61B5D9536");
pub const SILO_WRAPPER: Address = address!("5F127Aedf5A31E2F2685E49618D4f4809205fd62");

pub mod multipool {
    use super::sol;
    sol!(
        #[allow(missing_docs)]
        #[sol(rpc, all_derives=true)]
        MultipoolContract,
        "../arcanum-contracts/out/Multipool.sol/Multipool.json"
    );
}

pub mod trader {
    use super::sol;
    sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        Trader,
        "../arcanum-contracts/out/Trader.sol/Trader.json"
    );
}

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    IUniswapV3Pool,
    "../arcanum-contracts/out/IUniswapV3Pool.sol/IUniswapV3Pool.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Quoter,
    "../arcanum-contracts/out/IQuoterV2.sol/IQuoterV2.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract SiloPool {
        function deposit(address _asset, uint256 _amount, bool _collateralOnly) external returns (uint256 collateralAmount, uint256 collateralShare);
        function withdraw(address _asset, uint256 _amount, bool _collateralOnly) external returns (uint256 withdrawnAmount, uint256 withdrawnShare);
    }
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract SiloLens {
        function totalDepositsWithInterest(address _silo,address _asset) external view returns (uint256 _totalDeposits);
    }
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract ERC20 {
        function totalSupply() external view returns (uint256 value);
        function approve(address recepient,uint256 amount) external;
        function transfer(address recepient,uint256 amount) external;
    }
);
