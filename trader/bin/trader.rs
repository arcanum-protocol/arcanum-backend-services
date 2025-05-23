use backend_service::ServiceConfig;
use multipool_trader::TraderService;

fn main() {
    ServiceConfig::<TraderService>::from_file(
        std::env::var("CONFIG_PATH")
            .expect("CONFIG_PATH not set")
            .as_str(),
    )
    .initialize();
}
