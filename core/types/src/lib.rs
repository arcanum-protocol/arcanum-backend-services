use alloy::sol;

pub mod borsh_methods;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[allow(missing_docs)]
    #[sol(rpc)]
    Multipool,
    "Multipool.json"
);
