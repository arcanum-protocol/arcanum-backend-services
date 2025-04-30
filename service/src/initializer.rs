use crate::{
    health_check::init_health_check,
    logging::{LogTarget, ServiceTarget, METER_PROVIDER},
    ServiceConfig, ServiceData,
};

use log::{Level, Log};
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, Protocol, WithExportConfig};
use opentelemetry_sdk::{logs::LoggerProviderBuilder, metrics::SdkMeterProvider, Resource};

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
        let resource = Resource::builder()
            .with_attribute(KeyValue::new(
                "service.name",
                config.service_name.to_string(),
            ))
            .with_attribute(KeyValue::new(
                "service.environment",
                config.environment.to_string(),
            ))
            .build();

        let otel_log_provider = LoggerProviderBuilder::default()
            .with_batch_exporter(
                LogExporter::builder()
                    .with_tonic()
                    .with_protocol(Protocol::Grpc)
                    .with_endpoint(telemetry.otel_endpoint.to_string())
                    .with_timeout(Duration::from_millis(telemetry.otel_sync_interval))
                    .build()
                    .unwrap(),
            )
            .with_resource(resource.clone())
            .build();

        log::set_boxed_logger(Box::new(CombineLogger(
            pretty_env_logger::formatted_builder()
                .parse_default_env()
                .build(),
            OpenTelemetryLogBridge::new(&otel_log_provider),
        )))
        .expect("logging already initialized");
        log::set_max_level(Level::Info.to_level_filter());

        let metrics = SdkMeterProvider::builder()
            .with_periodic_exporter(
                MetricExporter::builder()
                    .with_tonic()
                    .with_protocol(Protocol::Grpc)
                    .with_endpoint(telemetry.otel_endpoint.to_string())
                    .with_timeout(Duration::from_millis(telemetry.otel_sync_interval))
                    .build()
                    .unwrap(),
            )
            .with_resource(resource)
            .build();

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
