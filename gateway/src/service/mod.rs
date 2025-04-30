pub mod log_target;
pub mod metrics;

pub mod termination_codes {
    pub const RPC_RETRY_ERROR: i32 = 0x01;
    pub const PRICE_FETCH_FAILED: i32 = 0x02;
}
