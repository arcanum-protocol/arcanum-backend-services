use std::{env, str::FromStr};

use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

#[get("/api/v1/health")]
async fn health() -> impl Responder {
    format!("ok")
}

use ethers::signers::Wallet;
use price_oracle::{config::BotConfig, crypto::sign, multipool_storage::MultipoolStorage};
use primitive_types::U128;
use serde::Deserialize;

#[derive(Deserialize)]
struct PriceRequest {
    multipool_id: String,
}

#[get("/oracle/v1/signed_price")]
async fn get_signed_price(
    params: web::Query<PriceRequest>,
    key: web::Data<String>,
    storage: web::Data<MultipoolStorage>,
) -> impl Responder {
    let signer = Wallet::from_str(&key).unwrap();
    let price = storage.get_signed_price(&params.multipool_id, &signer);
    HttpResponse::Ok().json(price)
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    let key = env::var("KEY").expect("KEY must be set");
    let config_path = env::var("CONFIG_PATH").expect("CONFIG_PATH must be set");

    let config = BotConfig::from_file(&config_path);
    let storage = MultipoolStorage::from_config(config);

    let jh = tokio::spawn(storage.gen_fetching_future());

    println!("starting server at {}", bind_address);
    let server = HttpServer::new(move || {
        let cors = Cors::permissive();
        let key = key.clone();
        let storage = storage.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(key))
            .app_data(web::Data::new(storage))
            .service(health)
            .service(get_signed_price)
    })
    .bind(bind_address)?
    .run();
    futures::future::join(jh, server).await;
    Ok(())
}
