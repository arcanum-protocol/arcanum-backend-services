use alloy::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Multipool,
    "src/abi/multipool.json"
);
