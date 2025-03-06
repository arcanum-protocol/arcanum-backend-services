use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use anyhow::anyhow;
use itertools::Itertools;
use multipool_storage::hook::HookInitializer;
use multipool_types::kafka::KafkaTopics;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct PriceFetcher<P: Provider + Clone + 'static> {
    pub producer: FutureProducer,
    pub delay: Duration,
    pub multicall_chunk_size: usize,
    pub multicall_address: Address,
    pub rpc: P,
}

impl<P: Provider + Clone + 'static> PriceFetcher<P> {
    pub async fn process_prices(
        &self,
        mp: Address,
        assets: Vec<Address>,
    ) -> anyhow::Result<HashMap<Address, U256>> {
        let multipool_functions = multipool_types::Multipool::abi::functions();
        let get_price_func = &multipool_functions.get("getPrice").unwrap()[0];

        let mut prices = Vec::new();
        let chunked_assets = assets
            .iter()
            .chunks(self.multicall_chunk_size)
            .into_iter()
            .map(|chunk| chunk.into_iter().collect_vec())
            .collect_vec();
        for chunk in chunked_assets {
            let mut mc = alloy_multicall::Multicall::new(&self.rpc, self.multicall_address);
            for asset in chunk {
                mc.add_call(mp, get_price_func, &[DynSolValue::Address(*asset)], true);
            }
            let result = mc
                .call()
                .await?
                .into_iter()
                .map(|p| p.unwrap().as_uint().unwrap().0);
            prices.extend(result);
        }
        Ok(assets.into_iter().zip(prices.into_iter()).collect())
    }
}

impl<P: Provider + Clone + 'static> HookInitializer for PriceFetcher<P> {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let instance = self.clone();
        vec![tokio::spawn(async move {
            loop {
                let mp = multipool();
                let mp_address = mp.contract_address();
                let asset_prices = instance.process_prices(mp_address, mp.asset_list()).await?;
                instance
                    .producer
                    .send(
                        // Somehow fix all transitions
                        FutureRecord::to(KafkaTopics::MpPrices.as_ref())
                            .key(&format!("{}{}", 1, mp.contract_address()))
                            .payload(
                                &serde_json::json!({
                                    "address": mp_address,
                                    "prices": asset_prices,
                                })
                                .to_string(),
                            ),
                        Duration::from_secs(1),
                    )
                    .await
                    .map_err(|(e, msg)| {
                        anyhow!("Failed to send log to kafka: {e} ; \n Message: {:?}", msg)
                    })?;
                tokio::time::sleep(instance.delay).await;
            }
        })]
    }
}
