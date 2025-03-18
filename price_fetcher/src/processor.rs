use alloy::providers::Provider;
use anyhow::anyhow;
use multipool_storage::hook::HookInitializer;
use multipool_storage::price_fetch::get_asset_prices;
use multipool_types::kafka::KafkaTopics;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use std::time::Duration;

#[derive(Clone)]
pub struct PriceFetcher<P: Provider + Clone + 'static> {
    pub producer: FutureProducer,
    pub delay: Duration,
    pub multicall_chunk_size: usize,
    pub rpc: P,
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
                let asset_prices = get_asset_prices(
                    mp_address,
                    mp.asset_list(),
                    instance.multicall_chunk_size,
                    &instance.rpc,
                )
                .await?;
                instance
                    .producer
                    .send(
                        // Somehow fix all transitions
                        FutureRecord::to(KafkaTopics::MpPrices.as_ref())
                            .key(&format!("{}|{}", 1, mp.contract_address()))
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
