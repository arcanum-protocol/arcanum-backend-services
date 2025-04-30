use backend_service::logging::LogTarget;

pub enum GatewayTarget {
    Api,
    Indexer,
    PriceFetcher,
    Rpc,
}

impl LogTarget for GatewayTarget {
    fn target(&self) -> &str {
        use GatewayTarget::*;
        match self {
            Api => "api",
            Indexer => "indexer",
            PriceFetcher => "price-fetcher",
            Rpc => "rpc",
        }
    }
}
