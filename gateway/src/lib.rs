use std::env;
use std::sync::Arc;
use std::time::Duration;

use alloy::providers::ProviderBuilder;
use alloy::transports::http::reqwest::Url;
use alloy::{primitives::Address, rpc::client::ClientBuilder};
use backend_service::ServiceData;
use cache::AppState;
use indexer1::Indexer;
use multipool::Multipool;
use price_fetcher::PriceFetcherConfig;
use routes::{charts, portfolio};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

//TODO: add arwave mathcnism
//TOOD: add oracle

use anyhow::Context;
use axum::{
    routing::{get, post},
    Router,
};

use crate::layers::api_metrics::OtelMetricsLayer;

pub mod cache;
pub mod error;
pub mod indexer;
pub mod layers;
pub mod price_fetcher;
pub mod routes;
pub mod service;

#[derive(Deserialize)]
pub struct IndexerConfig {
    from_block: u64,
    fetch_interval_ms: u64,
}

#[derive(Deserialize)]
pub struct DbConfig {
    env_key: Option<String>,
}

#[derive(Deserialize)]
pub struct RpcConfig {
    ws_url: Option<String>,
    http_url: String,
    max_retry: u32,
    backoff_ms: u64,
}

#[derive(Deserialize)]
pub struct ArweaveConfig {
    rpc_url: String,
    wallet_path: String,
}

#[derive(Deserialize)]
pub struct GatewayService {
    price_fetcher: PriceFetcherConfig,
    indexer: IndexerConfig,
    database: Option<DbConfig>,
    // TODO: make optional when use create2 to deploy factory
    factory: Address,
    bind_to: Option<String>,
    rpc: RpcConfig,
    arweave: Option<ArweaveConfig>,
}

impl ServiceData for GatewayService {
    async fn run(self) -> anyhow::Result<()> {
        let database_env_key = self
            .database
            .and_then(|d| d.env_key)
            .unwrap_or("DATABASE_URL".into());
        let database_url =
            env::var(&database_env_key).context(format!("{} must be set", database_env_key))?;
        let pool = sqlx::PgPool::connect(&database_url).await?;

        let retry_layer =
            layers::backoff::RetryBackoffLayer::new(self.rpc.max_retry, self.rpc.backoff_ms);

        let http_client = ClientBuilder::default()
            .layer(retry_layer)
            .http(Url::parse(&self.rpc.http_url).context("Failed to parse http rpc url")?);

        // Create a new provider with the client.
        let provider_http = ProviderBuilder::new().on_client(http_client);

        let app_state = Arc::new(
            AppState::initialize(pool.clone(), provider_http, self.factory, self.arweave)
                .await
                .unwrap(),
        );

        let price_fetcher_handle = price_fetcher::run(app_state.clone(), self.price_fetcher);

        let indexer_handle = {
            let processor = indexer::PgEventProcessor {
                app_state: app_state.clone(),
            };
            let pool = pool.clone();

            Indexer::builder()
                .pg_storage(pool)
                .http_rpc_url(self.rpc.http_url.parse()?)
                .ws_rpc_url_opt(self.rpc.ws_url.map(|url| url.parse()).transpose()?)
                .block_range_limit(999)
                .fetch_interval(Duration::from_millis(self.indexer.fetch_interval_ms))
                .filter(Multipool::filter().from_block(self.indexer.from_block))
                .set_processor(processor)
                .build()
                .await
                .map_err(|e| {
                    println!("indexer err {e:?}");
                    e
                })
                .context("Failed to build indexer")?
                .run()
        };

        let app = Router::new()
            .route("/portfolio/candles", get(charts::candles))
            .route("/portfolio/stats", get(charts::stats))
            .route("/portfolio/list", get(portfolio::list))
            .route("/portfolio/create", post(portfolio::create))
            .route("/portfolio/metadata", get(portfolio::metadata))
            .route(
                "/account/positions_history",
                get(portfolio::positions_history),
            )
            .route("/account/positions", get(portfolio::positions))
            .layer(OtelMetricsLayer)
            .layer(CorsLayer::permissive())
            .with_state(app_state);

        let listener =
            tokio::net::TcpListener::bind(self.bind_to.unwrap_or("0.0.0.0:8080".into())).await?;
        let axum = axum::serve(listener, app);
        tokio::select! {
            axum = axum => axum.map_err(Into::into),
            i = indexer_handle => i,
            p = price_fetcher_handle => p,
        }
    }
}
