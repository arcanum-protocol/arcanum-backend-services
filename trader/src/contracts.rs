use alloy::{
    primitives::{address, Address},
    sol,
};

pub const TRADER_ADDRESS: Address = address!("955aDe421294B9D9C49b09bd64a92a4138EA6C56");
pub const CASHBACK_VAULT: Address = address!("B9cb365F599885F6D97106918bbd406FE09b8590");

pub const WETH_ADDRESS: Address = address!("82aF49447D8a07e3bd95BD0d56f35241523fBab1");

pub const QUOTER_ADDRESS: Address = address!("b27308f9F90D607463bb33eA1BeBb41C27CE5AB6");
// ethereum mainnet -- must be mc3
pub const MULTICALL_ADDRESS: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");

pub const SILO_LENS: Address = address!("BDb843c7a7e48Dc543424474d7Aa63b61B5D9536");
pub const SILO_WRAPPER: Address = address!("5F127Aedf5A31E2F2685E49618D4f4809205fd62");

pub mod multipool {
    use super::sol;
    sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
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
    contract UniswapPool {
        function slot0() external view returns (uint160,int24,uint16,uint16,uint16,uint8,bool);
        function observe(uint32[] secondsAgos) external view returns (int56[],uint160[]);
    }
);

sol!(
    #[allow(missing_docs)]
    #[sol(abi)]
    contract Quoter {
        function quoteExactInputSingle(address tokenIn,address tokenOut,uint24 fee,uint256 amountIn,uint160 sqrtPriceLimitX96) external returns (uint256 amountOut);
        function quoteExactOutputSingle(address tokenIn,address tokenOut,uint24 fee,uint256 amountOut,uint160 sqrtPriceLimitX96) external returns (uint256 amountIn);
    }
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
