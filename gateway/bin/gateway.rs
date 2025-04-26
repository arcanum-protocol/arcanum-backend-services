use backend_service::ServiceConfig;
use gateway::GatewayService;

fn main() {
    ServiceConfig::<GatewayService>::from_file(
        std::env::var("CONFIG_PATH")
            .expect("CONFIG_PATH not set")
            .as_str(),
    )
    .initialize();
}
