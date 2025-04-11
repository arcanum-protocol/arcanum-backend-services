use backend_service::ServiceConfig;
use price_fetcher::PriceFetcherService;

fn main() {
    ServiceConfig::<PriceFetcherService>::from_file(
        std::env::var("CONFIG_PATH")
            .expect("CONFIG_PATH not set")
            .as_str(),
    )
    .initialize();
}
