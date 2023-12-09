use std::{env, iter::repeat, str::FromStr, sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

use futures::future::join_all;
use tokio::time::sleep;
use tokio_postgres::{Error, NoTls};

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
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .expect("Postres connect should be valid");

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let config = BotConfig::from_file(&config_path);
    let storage = MultipoolStorage::from_config(config);

    let jh = tokio::spawn(storage.gen_fetching_future());

    {
        let storage = storage.clone();
        let client = Arc::new(client);
        tokio::spawn(async move {
            loop {
                join_all(
                    storage
                        .get_prices()
                        .into_iter()
                        .zip(repeat(client.clone()))
                        .filter_map(|((id, price), client)| {
                            price.map(move |price| async move {
                                    client.execute(
                                    "call assemble_stats($1::TEXT, ($2::NUMBER/power(2::NUMBER,96)))",
                                    &[&id, &price.to_string()],
                                ).await.unwrap()
                            })
                        }),
                )
                .await;
                sleep(Duration::from_millis(100)).await;
            }
        });
    }

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
