use std::{fs, iter, path::PathBuf, sync::Arc, time::Duration};

use ethers::{
    prelude::{MULTICALL_ADDRESS, MULTICALL_SUPPORTED_CHAIN_IDS},
    providers,
    types::Address,
};
use futures::Future;
use ratelimit::Ratelimiter;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use rand::Rng;

pub struct Provider {
    pub parallel_calls: Option<Semaphore>,
    pub rate_limits: Option<Ratelimiter>,
    pub provider: Arc<providers::Provider<providers::Http>>,
}

impl Provider {
    fn providers_from_iter<I: IntoIterator<Item = RpcProviderParams>>(params: I) -> Arc<Vec<Self>> {
        Arc::new(
            params
                .into_iter()
                .map(|p| Provider {
                    parallel_calls: p.max_parallel_calls.map(Semaphore::new),
                    rate_limits: p.max_calls_per_sec.map(|m| {
                        Ratelimiter::builder(m, Duration::from_secs(1))
                            .max_tokens(m)
                            .build()
                            .expect("Failed to build rate limiter")
                    }),
                    provider: Arc::new(
                        providers::Provider::<providers::Http>::try_from(p.url)
                            .expect("Provider url should be valid"),
                    ),
                })
                .collect(),
        )
    }
}

impl Provider {
    pub async fn aquire<
        E,
        R,
        T: Future<Output = core::result::Result<R, E>> + Send,
        F: (Fn(Arc<providers::Provider<providers::Http>>, Option<Address>) -> T) + Send,
    >(
        &self,
        action: F,
        multicall: Option<Address>,
    ) -> core::result::Result<R, E> {
        let val = if let Some(parallel_calls) = self.parallel_calls.as_ref() {
            Some(
                parallel_calls
                    .acquire()
                    .await
                    .expect("No one ever closes it"),
            )
        } else {
            None
        };

        if let Some(rate_limits) = self.rate_limits.as_ref() {
            while let Err(duration) = rate_limits.try_wait() {
                tokio::time::sleep(duration).await;
            }
        }

        let r = action(self.provider.clone(), multicall).await;
        drop(val);
        r
    }
}

#[derive(Clone)]
pub struct RpcRobber {
    pub providers: Arc<Vec<Provider>>,
    pub multicall: Option<Address>,
    pub chain_id: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RpcProviderParams {
    max_parallel_calls: Option<usize>,
    max_calls_per_sec: Option<u64>,
    url: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RpcParams {
    providers: Vec<RpcProviderParams>,
    chain_id: u64,
    multicall_address: Option<Address>,
}

impl RpcRobber {
    pub fn from_anvil_mock(
        anvil_url: String,
        chain_id: u64,
        multicall_address: Option<Address>,
    ) -> Self {
        Self::from_params(RpcParams {
            providers: vec![RpcProviderParams {
                max_parallel_calls: None,
                max_calls_per_sec: None,
                url: anvil_url,
            }],
            chain_id,
            multicall_address,
        })
    }

    pub fn read(config_path: PathBuf) -> Self {
        let rpc_params: RpcParams = serde_yaml::from_slice(
            fs::read(config_path)
                .expect("Config should exist")
                .as_slice(),
        )
        .expect("Config should be valid");
        Self::from_params(rpc_params)
    }

    fn from_params(params: RpcParams) -> Self {
        Self {
            providers: Provider::providers_from_iter(params.providers),
            chain_id: params.chain_id,
            multicall: params
                .multicall_address
                .map(|v| Some(v))
                .unwrap_or_else(|| {
                    if MULTICALL_SUPPORTED_CHAIN_IDS.contains(&params.chain_id) {
                        Some(MULTICALL_ADDRESS)
                    } else {
                        None
                    }
                }),
        }
    }

    pub async fn aquire<
        E,
        R,
        T: Future<Output = core::result::Result<R, E>> + Send,
        F: (Fn(Arc<providers::Provider<providers::Http>>, Option<Address>) -> T) + Send + Sync,
    >(
        &self,
        action: F,
        retries: Option<usize>,
    ) -> core::result::Result<R, E> {
        let index = rand::thread_rng().gen::<usize>() % self.providers.len();
        let mut r = self.providers[index].aquire(&action, self.multicall).await;
        let retries = retries.unwrap_or(1);
        for _ in iter::repeat(()).take(retries) {
            if r.is_ok() {
                break;
            } else {
                r = self.providers[index].aquire(&action, self.multicall).await;
            }
        }
        r
    }
}
