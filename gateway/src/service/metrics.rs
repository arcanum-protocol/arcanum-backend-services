use backend_service::{
    global,
    metrics::{Gauge, Meter},
};

lazy_static::lazy_static! {
    pub static ref METER: Meter = global::meter("main");

    pub static ref TOTAL_API_REQUESTS: Gauge<u64> = METER.u64_gauge("total_api_requests").build();
    pub static ref API_REQUEST_DURATION_MS: Gauge<u64> = METER.u64_gauge("api_request_duration_ms").build();
    pub static ref TOTAL_FAILED_API_REQUESTS: Gauge<u64> = METER.u64_gauge("total_failed_api_requests").build();

    pub static ref DATABASE_REQUEST_DURATION_MS: Gauge<u64> = METER.u64_gauge("database_request_duration_ms").build();

    pub static ref INDEXED_LOGS_COUNT: Gauge<u64> = METER.u64_gauge("indexed_logs_count").build();
    pub static ref LOGS_COMMITEMENT_DURATION_MS: Gauge<u64> = METER.u64_gauge("logs_commitement_duration_ms").build();

    pub static ref TOTAL_RPC_REQUESTS: Gauge<u64> = METER.u64_gauge("total_rpc_requests").build();
    pub static ref TOTAL_FAILED_RPC_REQUESTS: Gauge<u64> = METER.u64_gauge("total_failed_rpc_requests").build();
    pub static ref RPC_REQUEST_DURATION_MS: Gauge<u64> = METER.u64_gauge("rpc_request_duration_ms").build();

    pub static ref PRICE_FETCHER_HEIGHT: Gauge<u64> = METER.u64_gauge("price_fetcher_height").build();
    pub static ref INDEXER_HEIGHT: Gauge<u64> = METER.u64_gauge("indexer_height").build();
}
