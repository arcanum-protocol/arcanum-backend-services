use alloy::{
    rpc::json_rpc::{RequestPacket, ResponsePacket},
    transports::{RpcError, TransportError},
};
use serde_json::json;

use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use tower::{Layer, Service};

use crate::service::{
    log_target::GatewayTarget::Rpc,
    metrics::{RPC_REQUEST_DURATION_MS, TOTAL_FAILED_RPC_REQUESTS, TOTAL_RPC_REQUESTS},
    termination_codes,
};
use backend_service::{logging::LogTarget, KeyValue};

#[derive(Debug, Clone)]
pub struct RetryBackoffLayer {
    max_rate_limit_retries: u32,
    backoff: u64,
}

impl RetryBackoffLayer {
    pub fn new(max_retries: u32, backoff_ms: u64) -> Self {
        Self {
            max_rate_limit_retries: max_retries,
            backoff: backoff_ms,
        }
    }
}

impl<S> Layer<S> for RetryBackoffLayer {
    type Service = RetryBackoffService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RetryBackoffService {
            inner,
            max_rate_limit_retries: self.max_rate_limit_retries,
            backoff: Duration::from_millis(self.backoff),
        }
    }
}

/// A Tower Service used by the RetryBackoffLayer that is responsible for retrying requests based
/// on the error type. See [TransportError] and [RateLimitRetryPolicy].
#[derive(Debug, Clone)]
pub struct RetryBackoffService<S> {
    /// The inner service
    inner: S,
    /// The maximum number of retries for rate limit errors
    max_rate_limit_retries: u32,
    /// The initial backoff in milliseconds
    backoff: Duration,
}

// Implement tower::Service for LoggingService.
impl<S: Clone + Send + 'static> Service<RequestPacket> for RetryBackoffService<S>
where
    // Constraints on the service.
    S: Service<RequestPacket, Response = ResponsePacket, Error = TransportError>,
    S::Future: Send + 'static,
    S::Response: Send + 'static + Debug,
    S::Error: Send + 'static + Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: RequestPacket) -> Self::Future {
        let inner = self.inner.clone();
        let this = self.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);
        Box::pin(async move {
            let mut rate_limit_retry_number: u32 = 0;
            loop {
                let request_types = match request {
                    RequestPacket::Single(ref r) => [r]
                        .into_iter()
                        .map(|r| KeyValue::new("rpc_method", r.method().to_string()))
                        .collect::<Vec<KeyValue>>(),
                    RequestPacket::Batch(ref br) => br
                        .into_iter()
                        .map(|r| KeyValue::new("rpc_method", r.method().to_string()))
                        .collect::<Vec<KeyValue>>(),
                };

                TOTAL_RPC_REQUESTS.record(1, &request_types);

                let timer = Instant::now();
                let res = inner.call(request.clone()).await;
                RPC_REQUEST_DURATION_MS.record(timer.elapsed().as_millis() as u64, &request_types);

                let err = match res {
                    Ok(res) => {
                        if let Some(e) = res.as_error() {
                            TransportError::ErrorResp(e.clone())
                        } else {
                            return Ok(res);
                        }
                    }
                    Err(e) => e,
                };

                TOTAL_FAILED_RPC_REQUESTS.record(1, &request_types);
                Rpc.error(json!({
                    "m": "returned call error",
                    "attempt_number": rate_limit_retry_number + 1,
                    "e": err.to_string(),
                }))
                .log();

                match &err {
                    RpcError::Transport(kind) if kind.is_retry_err() => {
                        rate_limit_retry_number += 1;

                        if rate_limit_retry_number > this.max_rate_limit_retries {
                            Rpc.error(json!({
                                "m": "max number of attemps exceeded, terminating service",
                            }))
                            .terminate(termination_codes::RPC_RETRY_ERROR);
                        }

                        tokio::time::sleep(this.backoff).await;
                    }
                    _ => {
                        return Err(err);
                    }
                }
            }
        })
    }
}
