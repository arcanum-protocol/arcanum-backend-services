use alloy::transports::BoxFuture;
use axum::{extract::Request, response::Response};
use backend_service::KeyValue;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

use crate::service::metrics::{
    API_REQUEST_DURATION_MS, TOTAL_API_REQUESTS, TOTAL_FAILED_API_REQUESTS,
};

#[derive(Clone)]
pub struct OtelMetricsLayer;

impl<S> Layer<S> for OtelMetricsLayer {
    type Service = OtelMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OtelMetricsService { inner }
    }
}

#[derive(Clone)]
pub struct OtelMetricsService<S> {
    inner: S,
}

impl<S> Service<Request> for OtelMetricsService<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let uri = request.uri().to_string();
        let future = self.inner.call(request);
        Box::pin(async move {
            let tags = &[KeyValue::new("method", uri)];
            TOTAL_API_REQUESTS.record(1, tags);

            let timer = Instant::now();
            let response: Response = future.await?;
            API_REQUEST_DURATION_MS.record(timer.elapsed().as_millis() as u64, tags);

            let status_code = response.status();
            if status_code.is_client_error() || status_code.is_server_error() {
                TOTAL_FAILED_API_REQUESTS.record(1, tags);
            }
            Ok(response)
        })
    }
}
