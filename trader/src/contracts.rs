use ethers::prelude::*;

pub mod multipool {
    use super::abigen;
    abigen!(MultipoolContract, "../core/storage/src/abi/multipool.json");
}

abigen!(
    UniswapPool,
    r#"[
        function slot0() external view returns (uint160,int24,uint16,uint16,uint16,uint8,bool)
        function observe(uint32[] secondsAgos) external view returns (int56[],uint160[])
    ]"#,
);

abigen!(
    Quoter,
    r#"[
        function quoteExactInputSingle(address tokenIn,address tokenOut,uint24 fee,uint256 amountIn,uint160 sqrtPriceLimitX96) external returns (uint256 amountOut)
        function quoteExactOutputSingle(address tokenIn,address tokenOut,uint24 fee,uint256 amountOut,uint160 sqrtPriceLimitX96) external returns (uint256 amountIn)
    ]"#,
);
