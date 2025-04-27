use backend_service::{
    global,
    metrics::{Meter, ObservableGauge},
};

lazy_static::lazy_static! {
    pub static ref METER: Meter = global::meter("main");

    pub static ref TOTAL_REQUESTS: ObservableGauge<u64> = METER.u64_observable_gauge("users_requests").init();
    pub static ref TOTAL_FAILED_REQUESTS: ObservableGauge<u64> = METER.u64_observable_gauge("failed_users_responses").init();

    pub static ref DATABASE_REQUEST_MS: ObservableGauge<u64> = METER.u64_observable_gauge("database_request_ms").init();
    pub static ref REQUEST_DURATION_MS: ObservableGauge<u64> = METER.u64_observable_gauge("api_request_ms").init();

    pub static ref INDEXED_LOGS_COUNT: ObservableGauge<u64> = METER.u64_observable_gauge("indexed_logs_count").init();
    //TODO: gas metrics for multicall
}
