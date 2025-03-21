use std::{fmt::Display, future::Future};

use anyhow::Result;

use serde::{de::DeserializeOwned, Deserialize};

pub mod health_check;
pub mod initializer;
pub mod logging;

pub use opentelemetry::{global, metrics, KeyValue};

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Dev,
    Prod,
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Prod => write!(f, "prod"),
        }
    }
}

pub trait ServiceData {
    fn run(self) -> impl Future<Output = Result<()>>;
}

#[derive(Deserialize)]
pub struct ServiceConfig<D: ServiceData> {
    health_check: Option<String>,
    telemetry: Option<TelemetryConfig>,
    config: D,
    service_name: String,
    environment: Environment,
}

impl<D: DeserializeOwned + ServiceData> ServiceConfig<D> {
    pub fn from_file(path: &str) -> Self {
        serde_yaml::from_slice(
            std::fs::read(path)
                .unwrap_or_else(|_| panic!("Failed to read config file at {}", path))
                .as_slice(),
        )
        .expect("Failed to deserialize config from yaml")
    }

    pub fn initialize(self) {
        initializer::initialize_rt(self)
    }
}

#[derive(Deserialize)]
pub struct TelemetryConfig {
    otel_endpoint: String,
    otel_sync_interval: u64,
}
