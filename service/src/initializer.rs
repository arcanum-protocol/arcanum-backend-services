use crate::{
    health_check::init_health_check,
    logging::{LogTarget, ServiceTarget, METER_PROVIDER},
    ServiceConfig, ServiceData,
};

use log::{Level, Log};
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    logs::Config,
    metrics::reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
    Resource,
};

use serde::de::DeserializeOwned;

use std::time::Duration;

pub fn initialize_rt<D: DeserializeOwned + ServiceData>(config: ServiceConfig<D>) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(initialize_telemetry_and_run(config));
}

// Logger for sending logs to both OTEL and stdout
pub struct CombineLogger<L1, L2>(pub L1, pub L2);

impl<L1: Log, L2: Log> Log for CombineLogger<L1, L2> {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        self.0.enabled(metadata) || self.1.enabled(metadata)
    }

    fn log(&self, record: &log::Record<'_>) {
        self.0.log(record);
        self.1.log(record);
    }

    fn flush(&self) {
        self.0.flush();
        self.1.flush();
    }
}

pub async fn initialize_telemetry_and_run<D: DeserializeOwned + ServiceData>(
    config: ServiceConfig<D>,
) {
    if let Some(telemetry) = config.telemetry {
        let resource = Resource::new(vec![
            KeyValue::new("service.name", config.service_name.to_string()),
            KeyValue::new("service.environment", config.environment.to_string()),
        ]);

        let log_exporter = opentelemetry_otlp::new_exporter()
            .http()
            .with_http_client(reqwest::Client::new())
            .with_endpoint(telemetry.otel_endpoint.to_string())
            .with_timeout(Duration::from_millis(telemetry.otel_sync_interval));

        let metrics_exporter = opentelemetry_otlp::new_exporter()
            .http()
            .with_http_client(reqwest::Client::new())
            .with_endpoint(telemetry.otel_endpoint.to_string())
            .with_timeout(Duration::from_millis(telemetry.otel_sync_interval));

        let p = opentelemetry_otlp::new_pipeline()
            .logging()
            .with_log_config(Config::default().with_resource(resource.clone()))
            .with_exporter(log_exporter)
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("Failed to bootstrap otel");

        let otel_log_appender = OpenTelemetryLogBridge::new(p.provider());
        log::set_boxed_logger(Box::new(CombineLogger(
            pretty_env_logger::formatted_builder()
                .parse_default_env()
                .build(),
            otel_log_appender,
        )))
        .expect("logging already initialized");
        log::set_max_level(Level::Info.to_level_filter());

        let metrics = opentelemetry_otlp::new_pipeline()
            .metrics(opentelemetry_sdk::runtime::Tokio)
            .with_resource(resource.clone())
            .with_exporter(metrics_exporter)
            .with_period(Duration::from_secs(3))
            .with_timeout(Duration::from_secs(10))
            .with_aggregation_selector(DefaultAggregationSelector::new())
            .with_temporality_selector(DefaultTemporalitySelector::new())
            .build()
            .expect("Failed to bootstrap OTEL metrics");
        global::set_meter_provider(metrics.clone());
        METER_PROVIDER
            .set(metrics)
            .expect("Poorly initialized meter provider");

        ServiceTarget.info("OTEL is up").log();
        ServiceTarget.info("Service is starting").log();

        let _health_check_handle = tokio::spawn(async {
            init_health_check(config.health_check.unwrap_or("0.0.0.0:3030".into())).await
        });

        let r = config.config.run().await;
        if let Err(e) = r {
            ServiceTarget.error(&e.to_string()).terminate(0x0);
        }
    } else {
        pretty_env_logger::init();
        ServiceTarget.info("Running without OTEL").log();
        ServiceTarget.info("Service is starting").log();

        let _health_check_handle = tokio::spawn(async {
            init_health_check(config.health_check.unwrap_or("0.0.0.0:3030".into())).await
        });

        let r = config.config.run().await;
        if let Err(e) = r {
            ServiceTarget.error(&e.to_string()).terminate(0x0);
        }
    };
}
