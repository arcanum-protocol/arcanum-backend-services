use std::{env, time::Duration};

use tokio::time::sleep;
use tokio_postgres::NoTls;

use multipool_tracker::{bootstrap, config::BotConfig};

#[tokio::main]
async fn main() -> std::io::Result<()> {
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
    let (storage, rpcs) = bootstrap::run(config.clone()).await;

    sleep(Duration::from_secs(10)).await;

    multipool_tracker::trader::run(&storage, rpcs[0].clone(), config, client).await;

    Ok(())
}
