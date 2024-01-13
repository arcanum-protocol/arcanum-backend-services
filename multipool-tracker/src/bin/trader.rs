use std::{env, iter::repeat, str::FromStr, sync::Arc, time::Duration};

use futures::future::join_all;
use serde_json::Value;
use tokio::time::sleep;
use tokio_postgres::NoTls;

use ethers::signers::Wallet;
use multipool_tracker::{config::BotConfig, multipool_storage::MultipoolStorage};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    //let bind_address = env::var("BIND_ADDRESS").unwrap_or("0.0.0.0:8080".into());
    //let key = env::var("KEY").expect("KEY must be set");
    let config_path = env::var("CONFIG_PATH").expect("CONFIG_PATH must be set");
    // let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
    //     .await
    //     .expect("Postres connect should be valid");

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    //tokio::spawn(async move {
    //    if let Err(e) = connection.await {
    //        eprintln!("connection error: {}", e);
    //        std::process::exit(0x0700);
    //    }
    //});

    let config = BotConfig::from_file(&config_path);
    let storage = MultipoolStorage::from_config(config.clone());

    let jh = tokio::spawn(storage.gen_fetching_future());
    sleep(Duration::from_secs(10)).await;
    //let jh = tokio::spawn(async {});

    let trader = tokio::spawn(async move { multipool_tracker::trader::run(storage, config).await });

    let _ = futures::future::join(jh, trader).await;
    Ok(())
}
