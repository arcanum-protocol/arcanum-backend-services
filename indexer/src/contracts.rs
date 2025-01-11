use alloy::sol;
use serde::Serialize;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Serialize, Debug)]
    Multipool,
    "src/abi/multipool.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Serialize, Debug)]
    MultipoolFactory,
    "src/abi/multipool_factory.json"
);
