use std::env;
use std::sync::Arc;

use alloy::primitives::Address;
use routes::portfolio;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use sqlx::{postgres::PgRow, Row};

pub mod cache;
pub mod routes;

#[tokio::main]
async fn main() {
    // initialize tracing
    //tracing_subscriber::fmt::init();
    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

    // build our application with a route
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
        .with_state(Arc::new(pool));
    // `GET /` goes to `root`

    // run our app with hyper, listening globally on port 3000
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
