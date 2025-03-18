use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::Result;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Response;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::logging::{LogTarget, ServiceTarget};

pub async fn init_health_check(health_check_bind_address: String) -> Result<()> {
    let addr = SocketAddr::from_str(health_check_bind_address.as_str())?;

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);
        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(
                    io,
                    service_fn(|_| async move {
                        anyhow::Ok(Response::new(Full::new(Bytes::from("ok"))))
                    }),
                )
                .await
            {
                ServiceTarget.error(&err.to_string()).terminate(0x332);
            }
        });
    }
}
