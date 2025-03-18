use std::time::Duration;

use once_cell::sync::OnceCell;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use serde::Serialize;

pub static METER_PROVIDER: OnceCell<SdkMeterProvider> = OnceCell::new();

#[derive(Default)]
pub struct ServiceTarget;

impl LogTarget for ServiceTarget {
    fn target(&self) -> &str {
        "service"
    }
}

pub trait LogTarget: Sized {
    fn target(&self) -> &str;

    fn error<D: Serialize>(self, error: D) -> ErrorLog<Self, D> {
        ErrorLog {
            target: self,
            error,
        }
    }

    fn info<D: Serialize>(self, message: D) -> InfoLog<Self, D> {
        InfoLog {
            target: self,
            message,
        }
    }
}

#[derive(Serialize)]
pub struct ErrorLog<T: LogTarget, D: Serialize> {
    target: T,
    error: D,
}

impl<T: LogTarget, D: Serialize> ErrorLog<T, D> {
    pub fn terminate(self, code: i32) -> ! {
        self.log();
        std::thread::sleep(Duration::from_secs(2));
        log::logger().flush();
        if let Some(metrics) = METER_PROVIDER.get() {
            let _ = metrics.force_flush();
        }
        std::process::exit(code)
    }

    pub fn log(self) {
        log::error!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "target": self.target.target(),
                "error": self.error,
            }))
            .expect("Log serialization never fails")
        );
    }
}

#[derive(Serialize)]
pub struct InfoLog<T: LogTarget, D: Serialize> {
    target: T,
    message: D,
}

impl<T: LogTarget, D: Serialize> InfoLog<T, D> {
    pub fn log(self) {
        log::info!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "target": self.target.target(),
                "message": self.message,
            }))
            .expect("Log serialization never fails")
        );
    }
}
