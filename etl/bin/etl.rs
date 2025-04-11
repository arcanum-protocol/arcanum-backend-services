use backend_service::ServiceConfig;
use etl::EtlService;

fn main() {
    ServiceConfig::<EtlService>::from_file(
        std::env::var("CONFIG_PATH")
            .expect("CONFIG_PATH not set")
            .as_str(),
    )
    .initialize();
}
