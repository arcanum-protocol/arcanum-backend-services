use std::{env, sync::Arc};

use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

use clap::Parser;
use multipool_cache::cache::CachedMultipoolData;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to ledger storage
    #[arg(short, long, default_value_t = String::from("./ledger"))]
    ledger: String,

    /// Path to config file
    #[arg(long)]
    rpc_config: Option<String>,

    #[arg(long)]
    uniswap_config: Option<String>,

    #[arg(long)]
    price_fetch_interval: Option<u64>,

    #[arg(long)]
    quantity_fetch_interval: Option<u64>,

    #[arg(long)]
    share_fetch_interval: Option<u64>,

    #[arg(long)]
    sync_interval: Option<u64>,

    #[arg(long)]
    monitoring_interval: Option<u64>,

    /// Path to config file
    #[arg(short, long, default_value_t = 8080)]
    bind_port: u64,
}

#[get("/health")]
async fn health() -> impl Responder {
    format!("ok")
}

use multipool_ledger::DiscLedger;
use multipool_trader::{trade::Uniswap, TraderHook};
use rpc_controller::RpcRobber;

use ethers::types::Address;
use multipool_storage::{builder::MultipoolStorageBuilder, MultipoolStorage};
use serde::Deserialize;
use tokio::runtime::Handle;

#[derive(Deserialize)]
struct MultipoolId {
    multipool_address: Address,
}

#[get("/signed_price")]
async fn get_signed_price(
    params: web::Query<MultipoolId>,
    cache: web::Data<Arc<CachedMultipoolData>>,
) -> impl Responder {
    cache
        .get_signed_price(&params.multipool_address)
        .map(|price| HttpResponse::Ok().json(price))
        .unwrap_or(HttpResponse::NotFound().json("Price not found"))
}

#[get("/asset_list")]
async fn get_asset_list(
    params: web::Query<MultipoolId>,
    storage: web::Data<MultipoolStorage<()>>,
) -> impl Responder {
    let mp = storage.get_pool(&params.multipool_address).await.unwrap();
    let mp = mp.read().await.clone();
    let assets = mp.multipool.asset_list();
    HttpResponse::Ok().json(assets)
}

#[get("/assets")]
async fn get_assets(
    params: web::Query<MultipoolId>,
    storage: web::Data<MultipoolStorage<()>>,
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

    let rpc = RpcRobber::read(args.rpc_config.unwrap().into());

    let cache = Arc::new(CachedMultipoolData::default());

    let hook = if let Some(path) = args.uniswap_config {
        let uniswap = Arc::new(Uniswap::try_from_file(path.into()));
        let weth: Address = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"
            .parse()
            .unwrap();
        let trader_hook = TraderHook {
            uniswap,
            cache: cache.clone(),
            rpc: rpc.clone(),
            handle: Handle::current(),
            weth,
        };
        Some(trader_hook)
    } else {
        None
    };

    let storage = MultipoolStorageBuilder::default()
        .ledger(
            DiscLedger::at(args.ledger.into())
                .await
                .expect("Failed to set up ledger"),
        )
        .rpc(rpc.clone())
        .target_share_interval(args.share_fetch_interval.unwrap_or(10000))
        .price_interval(args.price_fetch_interval.unwrap_or(1000))
        .ledger_sync_interval(args.sync_interval.unwrap_or(500))
        .quantity_interval(args.quantity_fetch_interval.unwrap_or(1000))
        .monitoring_interval(args.monitoring_interval.unwrap_or(1000))
        .set_hook(hook)
        .build()
        .await
        .expect("Failed to build storage");

    {
        let cache = cache.clone();
        let storage = storage.clone();
        tokio::spawn(async move {
            cache
                .clone()
                .refresh(storage, 5000, 180, rpc.chain_id.into(), key.clone())
                .await
        });
    }

    HttpServer::new(move || {
        let cors = Cors::permissive();
        //let key = key.clone();
        let storage = storage.clone();
        let cache = cache.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(cache))
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
