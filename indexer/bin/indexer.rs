use backend_service::ServiceConfig;
use multipool_indexer::IndexerService;

fn main() {
    ServiceConfig::<IndexerService>::from_file(
        std::env::var("CONFIG_PATH")
            .expect("CONFIG_PATH not set")
            .as_str(),
    )
    .initialize();
}
