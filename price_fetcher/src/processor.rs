use alloy::providers::Provider;
use anyhow::anyhow;
use multipool_storage::{hook::HookInitializer, price_fetch::get_asset_prices};
use multipool_types::{
    expiry::{MayBeExpired, StdTimeExtractor},
    messages::{KafkaTopics, MsgPack, PriceData},
};
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

#[derive(Clone)]
pub struct PriceFetcher<P: Provider + Clone + 'static> {
    pub producer: FutureProducer,
    pub delay: Duration,
    pub multicall_chunk_size: usize,
    pub rpc: P,
    //TOOD: in a lot of places chain id can be gained from RPC
    pub chain_id: u64,
}

impl<P: Provider + Clone + 'static> HookInitializer for PriceFetcher<P> {
    async fn initialize_hook<F: Fn() -> multipool::Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> Vec<tokio::task::JoinHandle<anyhow::Result<()>>> {
        let chain_id = self.chain_id.clone();
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
                let data = PriceData {
                    address: mp_address,
                    prices: asset_prices
                        .into_iter()
                        .map(|(address, price)| {
                            (address, MayBeExpired::build::<StdTimeExtractor>(price))
                        })
                        .collect(),
                };
                instance
                    .producer
                    .send(
                        // Somehow fix all transitions
                        FutureRecord::to(KafkaTopics::MpPrices(chain_id).to_string().as_str())
                            .key(&format!("{}|{}", 1, mp.contract_address()))
                            .payload(&data.pack()),
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
