use std::env;
use std::sync::Arc;

use alloy::primitives::Address;
use dashmap::DashMap;
use multipool_storage::kafka::into_fetching_task;
use multipool_storage::{hook::HookInitializer, storage::MultipoolStorage};
use multipool_types::messages::KafkaTopics;
use multipool_types::FACTORY_ADDRESS;
use routes::portfolio;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::FutureProducer,
    ClientConfig,
};
use sqlx::{postgres::PgRow, PgPool, Row};

pub mod cache;
pub mod routes;

#[derive(Clone, Default)]
pub struct MultipoolGettersStorage {
    pub getters:
        DashMap<Address, Arc<Box<dyn Fn() -> multipool::Multipool + Send + Sync + 'static>>>,
}

impl HookInitializer for MultipoolGettersStorage {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let address = multipool().contract_address();
        self.getters.insert(address, Arc::new(Box::new(multipool)));
        vec![]
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub getters: MultipoolGettersStorage,
}

#[tokio::main]
async fn main() {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let kafka_group = env::var("KAFKA_GROUP").expect("KAFKA_GROUP must be set");
    let kafka_url = env::var("KAFKA_URL").expect("KAFKA_URL must be set");
    // can get from postgres??
    let chain_ids: Vec<u64> =
        serde_json::from_str(&env::var("CHAIN_IDS").expect("CHAIN_IDS must be set")).unwrap();

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

    let getters_hook = MultipoolGettersStorage::default();

    let db = sled::open("sled_db").unwrap();
    let mut storage = MultipoolStorage::init(db, getters_hook.clone(), FACTORY_ADDRESS)
        .await
        .unwrap();

    tokio::spawn(async move {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &kafka_group)
            .set("bootstrap.servers", &kafka_url)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Creation failed");

        let topics = &chain_ids
            .into_iter()
            .map(|chain_id| {
                vec![
                    KafkaTopics::ChainEvents(chain_id).to_string(),
                    KafkaTopics::MpPrices(chain_id).to_string(),
                ]
                .into_iter()
            })
            .flatten()
            .collect::<Vec<String>>();
        consumer
            .subscribe(
                topics
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .as_slice(),
            )
            .expect("Failed to subscribe to topic");

        into_fetching_task(&mut storage, consumer).await.unwrap();
    });

    let app_state = AppState {
        pool,
        getters: getters_hook,
    };

    let app = Router::new()
        .route("/charts/history", get(routes::charts::history))
        .route("/charts/stats", get(routes::charts::stats))
        .route("/portfolio/list", get(portfolio::list))
        .route("/portfolio", get(portfolio::portfolio))
        .route("/portfolio/create", post(portfolio::create))
        //.route("/assets/list", get(history))
        //.route("/account/positions", get(history))
        //.route("/account/history", get(history))
        //.route("/account/pnl", get(history))
        //.route("/chains", get(history))
        .with_state(Arc::new(app_state));

    let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
pub struct AssetsRequest {
    chain_id: i32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Asset {
    address: Address,
    symbol: String,
    name: String,
    decimals: u8,
    logo_url: Option<String>,
    twitter_url: Option<String>,
    description: Option<String>,
    website_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UniswapPool {
    asset_address: Address,
    pool_address: Address,
    base_is_asset0: bool,
    fee: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SiloPool {
    asset_address: Address,
    base_asset_address: Address,
    pool_address: Address,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AssetsResponse {
    assets: Vec<Asset>,
    uniswap_pools: Vec<UniswapPool>,
    silo_pools: Vec<SiloPool>,
}

//#[actix_web::main]
//async fn main() -> std::io::Result<()> {
//    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
//    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
//    let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
//        .await
//        .expect("Postres connect should be valid");
//    tokio::spawn(async move {
//        if let Err(e) = connection.await {
//            println!("connection error: {}", e);
//            std::process::exit(0x0700);
//        }
//    });
//    let client = Arc::new(client);
//    HttpServer::new(move || {
//        let cors = Cors::permissive();
//        let client = client.clone();
//        App::new()
//            .wrap(cors)
//            .app_data(web::Data::new(client))
//            .service(config)
//            .service(symbols)
//            .service(history)
//            .service(stats)
//            .service(assets)
//    })
//    .bind(bind_address)?
//    .run()
//    .await
//}
