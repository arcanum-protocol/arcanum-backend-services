use std::{env, iter::repeat, str::FromStr, sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

use futures::future::join_all;
use serde_json::Value;
use tokio::time::sleep;
use tokio_postgres::NoTls;

#[get("/api/v1/health")]
async fn health() -> impl Responder {
    format!("ok")
}

use ethers::signers::Wallet;
use price_oracle::{config::BotConfig, multipool_storage::MultipoolStorage};
use serde::Deserialize;

#[derive(Deserialize)]
struct PriceRequest {
    multipool_id: String,
}

#[get("/oracle/v1/signed_price")]
async fn get_signed_price(
    params: web::Query<PriceRequest>,
    key: web::Data<String>,
    config: web::Data<BotConfig>,
    storage: web::Data<MultipoolStorage>,
) -> impl Responder {
    let signer = Wallet::from_str(&key).unwrap();
    let price = storage.get_signed_price(&params.multipool_id, &signer, config.poison_time);
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
            std::process::exit(0x0700);
        }
    });

    let config = BotConfig::from_file(&config_path);
    let storage = MultipoolStorage::from_config(config.clone());

    let jh = tokio::spawn(storage.gen_fetching_future());

    {
        let storage = storage.clone();
        let client = Arc::new(client);
        tokio::spawn(async move {
            sleep(Duration::from_millis(5000)).await;
            loop {
                let eth_price = reqwest::get(
                    "https://token-rates-aggregator.1inch.io/v1.0/native-token-rate?vs=USD",
                )
                .await
                .unwrap()
                .json::<Value>()
                .await
                .unwrap()
                .get("1")
                .expect("KEY \"1\" should present")
                .get("USD")
                .expect("KEY \"USD\" should present")
                .as_f64()
                .expect("Value should be a valid float");
                join_all(
                    storage
                        .get_prices(config.poison_time)
                        .into_iter()
                        .zip(repeat(client.clone()))
                        .filter_map(|((id, price), client)| {
                            price.map(move |price| async move {
                                    client.execute(
                                    "call assemble_stats($1::TEXT, ($2::TEXT::NUMERIC*$3::TEXT::NUMERIC/power(2::NUMERIC,96)))",
                                    &[&id, &price.to_string(), &eth_price.to_string()],
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
        let config = config.clone();
        let cors = Cors::permissive();
        let key = key.clone();
        let storage = storage.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(key))
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(storage))
            .service(health)
            .service(get_signed_price)
    })
    .bind(bind_address)?
    .run();
    let _ = futures::future::join(jh, server).await;
    Ok(())
}
