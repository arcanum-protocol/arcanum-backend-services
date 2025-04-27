use backend_service::logging::LogTarget;

pub enum GatewayTarget {
    Api,
    Indexer,
    PriceFetcher,
}

impl LogTarget for GatewayTarget {
    fn target(&self) -> &str {
        use GatewayTarget::*;
        match self {
            Api => "Api",
            Indexer => "indexer",
            PriceFetcher => "price fetcher",
        }
    }
}
