use std::{iter, sync::Arc, time::Duration};

use ethers::providers;
use futures::Future;
use ratelimit::Ratelimiter;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use rand::Rng;

pub struct Provider {
    pub parallel_calls: Semaphore,
    pub rate_limits: Ratelimiter,
    pub provider: Arc<providers::Provider<providers::Http>>,
}

impl Provider {
    pub async fn aquire<
        E,
        R,
        T: Future<Output = core::result::Result<R, E>> + Send,
        F: (Fn(Arc<providers::Provider<providers::Http>>) -> T) + Send,
    >(
        &self,
        action: F,
    ) -> core::result::Result<R, E> {
        let val = self
            .parallel_calls
            .acquire()
            .await
            .expect("No one ever closes it");
        while let Err(duration) = self.rate_limits.try_wait() {
            tokio::time::sleep(duration).await;
        }
        let r = action(self.provider.clone()).await;
        drop(val);
        r
    }
}

#[derive(Clone)]
pub struct RpcRobber {
    pub providers: Arc<Vec<Provider>>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RpcParams {
    max_parallel_calls: usize,
    max_calls_per_sec: u64,
    url: String,
}

impl RpcRobber {
    pub fn new<I: IntoIterator<Item = RpcParams>>(params: I) -> Self {
        Self {
            providers: Arc::new(
                params
                    .into_iter()
                    .map(|p| Provider {
                        parallel_calls: Semaphore::new(p.max_parallel_calls),
                        rate_limits: Ratelimiter::builder(
                            p.max_calls_per_sec,
                            Duration::from_secs(1),
                        )
                        .max_tokens(p.max_calls_per_sec)
                        .build()
                        .expect("Failed to build rate limiter"),
                        provider: Arc::new(
                            providers::Provider::<providers::Http>::try_from(p.url)
                                .expect("Provider url should be valid"),
                        ),
                    })
                    .collect(),
            ),
        }
    }

    pub async fn aquire<
        E,
        R,
        T: Future<Output = core::result::Result<R, E>> + Send,
        F: (Fn(Arc<providers::Provider<providers::Http>>) -> T) + Send + Sync,
    >(
        &self,
        action: F,
        retries: Option<usize>,
    ) -> core::result::Result<R, E> {
        let index = rand::thread_rng().gen::<usize>() % self.providers.len();
        let mut r = self.providers[index].aquire(&action).await;
        let retries = retries.unwrap_or(1);
        for _ in iter::repeat(()).take(retries) {
            if r.is_ok() {
                break;
            } else {
                r = self.providers[index].aquire(&action).await;
            }
        }
        r
    }
}
