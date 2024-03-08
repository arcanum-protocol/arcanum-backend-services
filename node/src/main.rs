use std::{env, str::FromStr};

use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use url::Url;

use clap::Parser;

pub mod crypto;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to ledger storage
    #[arg(short, long, default_value_t = String::from("./ledger/"))]
    ledger: String,

    /// Path to config file
    #[arg(short, long)]
    rpc_config: Option<String>,

    #[arg(long)]
    price_fetch_interval: u64,

    #[arg(long)]
    quantity_fetch_interval: u64,

    #[arg(long)]
    share_fetch_interval: u64,

    #[arg(long)]
    sync_interval: u64,

    /// Path to config file
    #[arg(short, long, default_value_t = 8080)]
    bind_port: u64,
}

#[get("/health")]
async fn health() -> impl Responder {
    format!("ok")
}

use multipool_ledger::DiscLedger;
use rpc_controller::RpcRobber;

use ethers::{signers::Wallet, types::Address};
use multipool_storage::{builder::MultipoolStorageBuilder, MultipoolStorage};
use serde::Deserialize;

#[derive(Deserialize)]
struct MultipoolId {
    multipool_address: Address,
}

#[get("/signed_price")]
async fn get_signed_price(
    params: web::Query<MultipoolId>,
    key: web::Data<String>,
    storage: web::Data<MultipoolStorage>,
) -> impl Responder {
    let signer = Wallet::from_str(&key).unwrap();
    let mp = storage.get_pool(&params.multipool_address).await.unwrap();
    let mp = mp.read().await.clone();
    let price = mp
        .multipool
        .get_price(&params.multipool_address)
        .unwrap()
        .not_older_than(180)
        .unwrap();
    let price = crypto::sign(params.multipool_address, price, 1, &signer);
    HttpResponse::Ok().json(price)
}

#[get("/asset_list")]
async fn get_asset_list(
    params: web::Query<MultipoolId>,
    storage: web::Data<MultipoolStorage>,
) -> impl Responder {
    let mp = storage.get_pool(&params.multipool_address).await.unwrap();
    let mp = mp.read().await.clone();
    let assets = mp.multipool.asset_list();
    HttpResponse::Ok().json(assets)
}

#[get("/assets")]
async fn get_assets(
    params: web::Query<MultipoolId>,
    storage: web::Data<MultipoolStorage>,
) -> impl Responder {
    let mp = storage.get_pool(&params.multipool_address).await.unwrap();
    let mp = mp.read().await.clone();
    let assets = mp.multipool.asset_list();
    HttpResponse::Ok().json(assets)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let args = Args::parse();
    let key = env::var("KEY").expect("KEY must be set");

    let storage = MultipoolStorageBuilder::default()
        .ledger(
            DiscLedger::at(args.ledger.into())
                .await
                .expect("Failed to set up ledger"),
        )
        .rpc(RpcRobber::read(args.rpc_config.unwrap().into()))
        .target_share_interval(args.share_fetch_interval)
        .price_interval(args.price_fetch_interval)
        .ledger_sync_interval(args.sync_interval)
        .quantity_interval(args.quantity_fetch_interval)
        .build()
        .await
        .expect("Failed to build storage");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        let key = key.clone();
        let storage = storage.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(key))
            .app_data(web::Data::new(storage))
            .service(health)
            .service(get_signed_price)
            .service(get_asset_list)
            .service(get_assets)
    })
    .bind(format!("0.0.0.0:{}", args.bind_port))?
    .run()
    .await
}
