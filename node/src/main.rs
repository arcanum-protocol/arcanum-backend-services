use std::{env, iter::repeat, sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

use anyhow::anyhow;
use clap::Parser;
use futures::{future::join_all, FutureExt};
use multipool_cache::cache::CachedMultipoolData;

use log::Level;
use opentelemetry::KeyValue;
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::Config;
use opentelemetry_sdk::Resource;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to ledger storage
    #[arg(short, long, default_value_t = String::from("./ledger/"))]
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

    #[arg(long)]
    otel_endpoint: Option<String>,

    #[arg(long)]
    otel_sync_interval: Option<u64>,

    /// Path to config file
    #[arg(short, long, default_value_t = 8080)]
    bind_port: u64,
}

#[get("/health")]
async fn health() -> impl Responder {
    "ok"
}

use multipool_ledger::DiscLedger;
use multipool_trader::{trade::Uniswap, TraderHook};
use rpc_controller::RpcRobber;

use ethers::types::Address;
use multipool_storage::{builder::MultipoolStorageBuilder, MultipoolStorage, StorageEntry};
use serde::Deserialize;
use tokio::{runtime::Handle, time::sleep};
use tokio_postgres::NoTls;

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
    cache: web::Data<Arc<CachedMultipoolData>>,
) -> impl Responder {
    let assets = cache
        .get_pool(&params.multipool_address)
        .unwrap()
        .asset_list();
    HttpResponse::Ok().json(assets)
}

#[get("/assets")]
async fn get_assets(cache: web::Data<Arc<CachedMultipoolData>>) -> impl Responder {
    let pools = cache.get_pools();
    HttpResponse::Ok().json(
        pools
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "pool": p,
                    "cap": p.cap().ok(),
                })
            })
            .collect::<Vec<serde_json::Value>>(),
    )
}

#[get("/bootstrap")]
async fn bootstrap(storage: web::Data<MultipoolStorage<TraderHook>>) -> impl Responder {
    HttpResponse::Ok().json(storage.build_ir().await)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let _p = if let Some(otel_endpoint) = args.otel_endpoint {
        let p = opentelemetry_otlp::new_pipeline()
            .logging()
            .with_log_config(Config::default().with_resource(Resource::new(vec![
                KeyValue::new("service.name", "arcanum-node"),
                KeyValue::new("service.chain", "arbitrum"),
            ])))
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .http()
                    .with_http_client(reqwest::Client::new())
                    .with_endpoint(otel_endpoint)
                    .with_timeout(Duration::from_millis(
                        args.otel_sync_interval.unwrap_or(1000),
                    )),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("Failed to bootstrap otel");

        let otel_log_appender = OpenTelemetryLogBridge::new(p.provider());
        log::set_boxed_logger(Box::new(otel_log_appender)).unwrap();
        log::set_max_level(Level::Info.to_level_filter());
        Some(p)
    } else {
        None
    };
    log::info!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "target": "app",
            "message": "Service is starting",
        }))
        .unwrap()
    );

    let key = env::var("KEY").expect("KEY must be set");
    let database_url = env::var("DATABASE_URL");

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
        .price_interval(args.price_fetch_interval.unwrap_or(5000))
        .ledger_sync_interval(args.sync_interval.unwrap_or(500))
        .quantity_interval(args.quantity_fetch_interval.unwrap_or(5000))
        .monitoring_interval(args.monitoring_interval.unwrap_or(5000))
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

    if let Ok(database_url) = database_url {
        let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
            .await
            .expect("Postres connect should be valid");
        let client = Arc::new(client);

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "target": "postgres-connection",
                        "error": format!("{e:?}"),
                        "message": "Connection error"
                    }))
                    .unwrap()
                );
                std::process::exit(0x0700);
            }
        });

        {
            // Add retry here
            let storage = storage.clone();
            tokio::spawn(async move {
                sleep(Duration::from_millis(500)).await;
                loop {
                    let request = reqwest::get(
                        //"https://token-rates-aggregator.1inch.io/v1.0/native-token-rate?vs=USD",
                        //"https://min-api.cryptocompare.com/data/price?fsym=ETH&tsyms=USD",
                        "https://api.binance.com/api/v3/avgPrice?symbol=ETHFDUSD",
                    )
                    .await;
                    let request = match request {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!(
                                "{}",
                                serde_json::to_string(&serde_json::json!({
                                    "target": "price-fetcher",
                                    "error": format!("{e:?}"),
                                    "message": "Failed to make eth price request"
                                }))
                                .unwrap()
                            );
                            std::process::exit(0x73);
                        }
                    };

                    //let eth_price = request
                    //    .json::<serde_json::Value>()
                    //    .await
                    //    .as_ref()
                    //    .map_err(|e| anyhow!("{e}"))
                    //    .and_then(|v| v.get("1").ok_or(anyhow!("KEY \"1\" should present")))
                    //    .and_then(|v| v.get("USD").ok_or(anyhow!("KEY \"USD\" should present")))
                    //    .and_then(|v| v.as_f64().ok_or(anyhow!("Value should be a valid float")));

                    let eth_price = request
                        .json::<serde_json::Value>()
                        .await
                        .as_ref()
                        .map_err(|e| anyhow!("{e}"))
                        .and_then(|v| v.get("price").ok_or(anyhow!("KEY \"USD\" should present")))
                        .and_then(|v| v.as_str().ok_or(anyhow!("Value should be a valid float")))
                        .and_then(|v| {
                            v.parse()
                                .map_err(|_| anyhow!("Failed to parse price to float"))
                        });

                    let eth_price: f64 = match eth_price {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!(
                                "{}",
                                serde_json::to_string(&serde_json::json!({
                                    "target": "price-fetcher",
                                    "error": format!("{e:?}"),
                                    "message": "Failed to parse eth price"
                                }))
                                .unwrap()
                            );
                            std::process::exit(0x74);
                        }
                    };

                    let client = client.clone();
                    storage
                    .pools()
                    .then(move |p| {
                        join_all(p.into_iter().zip(repeat(client.clone())).map(
                            move |(StorageEntry { multipool, address }, client)| async move {
                                let price = multipool
                                    .read()
                                    .await
                                    .multipool
                                    .get_price(&address)
                                    .map_err(|e| anyhow!("{e:?}"))?
                                    .not_older_than(180)
                                    .ok_or(anyhow!("Price expired"))?;
                                client
                                    .execute(
                                        "call assemble_stats(\
                                            $1::TEXT,\
                                            ($2::TEXT::NUMERIC*$3::TEXT::NUMERIC/power(2::NUMERIC,96))\
                                        )",
                                        &[
                                            &serde_json::to_string(&address)
                                            .expect("Addres serialization should be correct").trim_matches('\"'),
                                            &price.to_string(),
                                            &eth_price.to_string(),
                                        ],
                                    )
                                    .await
                                    .unwrap();
                                anyhow::Ok(())
                            },
                        ))
                    })
                    .then(|res| {
                        for r in res.iter() {
                            if let Err(e) = r {
                                log::error!(
                                    "{}",
                                    serde_json::to_string(&serde_json::json!({
                                        "target": "price-fetcher",
                                        "error": format!("{e:?}"),
                                        "message": "Failed to set price"
                                    }))
                                    .unwrap()
                                );
                            }
                        }

                        sleep(Duration::from_millis(1000))
                    })
                    .await;
                }
            });
        }
    } else {
        log::info!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "target": "postgres-connection",
                "message": "Running without database"
            }))
            .unwrap()
        );
    }

    HttpServer::new(move || {
        let cors = Cors::permissive();
        let storage = storage.clone();
        let cache = cache.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(cache))
            .app_data(web::Data::new(storage))
            .service(health)
            .service(get_signed_price)
            .service(bootstrap)
            .service(get_asset_list)
            .service(get_assets)
    })
    .bind(format!("0.0.0.0:{}", args.bind_port))?
    .run()
    .await
}
