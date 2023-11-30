use std::{env, str::FromStr};

use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

#[get("/api/v1/health")]
async fn health() -> impl Responder {
    format!("ok")
}

use ethers::signers::Wallet;
use price_oracle::MultipoolState;
use primitive_types::U128;
use serde::Deserialize;

#[derive(Deserialize)]
struct PriceRequest {
    contract_address: String,
}

#[get("/oracle/v1/signed_price")]
async fn get_signed_price(
    params: web::Query<PriceRequest>,
    key: web::Data<String>,
) -> impl Responder {
    let signer = Wallet::from_str(&key).unwrap();
    let r = MultipoolState::from_address(params.contract_address.clone())
        .sign(U128::from(7922816251426433759354395033u128), &signer)
        .await;
    HttpResponse::Ok().json(r)
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    let key: String = env::var("KEY").expect("KEY must be set");
    println!("starting server at {}", bind_address);
    HttpServer::new(move || {
        let cors = Cors::permissive();
        let key = key.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(key))
            .service(health)
            .service(get_signed_price)
    })
    .bind(bind_address)?
    .run()
    .await
}
