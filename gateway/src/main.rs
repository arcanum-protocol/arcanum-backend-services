use std::env;
use std::sync::Arc;
use std::time::Duration;

use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy::transports::http::reqwest::Url;
use cache::AppState;
use dashmap::DashMap;
use indexer1::Indexer;
use multipool::Multipool;
use multipool_storage::kafka::into_fetching_task;
use multipool_storage::{hook::HookInitializer, storage::MultipoolStorage};
use multipool_types::messages::KafkaTopics;
use multipool_types::FACTORY_ADDRESS;
use price_fetcher::PriceFetcherConfig;
use routes::{account, portfolio};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

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
pub mod error;
pub mod indexer;
pub mod price_fetcher;
pub mod routes;

#[derive(Clone, Default)]
pub struct MultipoolGettersStorage {
    pub getters:
        Arc<DashMap<Address, Arc<Box<dyn Fn() -> multipool::Multipool + Send + Sync + 'static>>>>,
}

impl HookInitializer for MultipoolGettersStorage {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let address = multipool().contract_address();
        println!("Got mp {address}");
        self.getters.insert(address, Arc::new(Box::new(multipool)));
        vec![]
    }
}

#[tokio::main]
async fn main() {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    let ws_rpc_url = env::var("WS_RPC_URL").expect("WS_RPC_URL must be set");
    let from_block: u64 = env::var("FROM_BLOCK")
        .expect("FROM_BLOCK must be set")
        .parse()
        .unwrap();

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

    let provider = ProviderBuilder::new().on_http(Url::parse(&rpc_url).unwrap());
    let app_state = Arc::new(AppState::initialize(pool.clone(), provider).await.unwrap());

    let price_fetcher_config = PriceFetcherConfig {
        block_delay: 3,
        multipools_in_chunk: 50,
        retry_delay_ms: 3000,
    };

    let price_fetcher_handle =
        tokio::spawn(price_fetcher::run(app_state.clone(), price_fetcher_config));

    let indexer_handle = {
        let processor = indexer::PgEventProcessor {
            app_state: app_state.clone(),
        };
        let pool = pool.clone();

        tokio::spawn(async move {
            Indexer::builder()
                .pg_storage(pool)
                .http_rpc_url(rpc_url.parse().unwrap())
                .ws_rpc_url(ws_rpc_url.parse().unwrap())
                .fetch_interval(Duration::from_millis(2000))
                .filter(Multipool::filter().from_block(from_block))
                .set_processor(processor)
                .build()
                .await
                .unwrap()
                .run()
                .await
        })
    };

    // init indexer
    // run all bros

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/charts/history", get(routes::charts::history))
        .route("/charts/stats", get(routes::charts::stats))
        .route("/portfolio/list", get(portfolio::list))
        .route("/portfolio", get(portfolio::portfolio))
        .route("/portfolio/create", post(portfolio::create))
        .route("/account/positions", get(account::positions))
        .route("/account/history", get(account::positions))
        .route("/account/pnl", get(account::pnl))
        // .route("/chains", get(history)) // do we really
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
