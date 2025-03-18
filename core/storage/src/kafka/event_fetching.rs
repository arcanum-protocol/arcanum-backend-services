use std::collections::HashMap;

use crate::hook::HookInitializer;
use crate::storage::MultipoolStorage;
use alloy::primitives::{Address, U256};
use anyhow::{anyhow, Context};
use futures::StreamExt;
use multipool::expiry::{EmptyTimeExtractor, MayBeExpired};
use multipool_types::kafka::{ChainBlock, KafkaTopics};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::Message;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PriceData {
    address: Address,
    // revrite to Vec<address, price>
    prices: HashMap<Address, MayBeExpired<U256, EmptyTimeExtractor>>,
}

pub async fn into_fetching_task<HI: HookInitializer>(
    storage: &mut MultipoolStorage<HI>,
    consumer: StreamConsumer,
) -> anyhow::Result<()> {
    loop {
        let mut stream = consumer.stream();
        // add better error handling
        while let Some(Ok(message)) = stream.next().await {
            match message.topic().into() {
                KafkaTopics::ChainEvents => {
                    let bytes = message
                        .payload()
                        .context(anyhow!("Received message with no payload"))?;
                    let blocks = serde_json::from_slice::<ChainBlock>(bytes).unwrap();
                    storage.apply_events(vec![blocks]).await?;
                }
                KafkaTopics::MpPrices => {
                    let bytes = message
                        .payload()
                        .context(anyhow!("Received message with no payload"))?;
                    let data = serde_json::from_slice::<PriceData>(bytes).unwrap();
                    storage.apply_prices(data.address, data.prices).await?;
                }
            }
            consumer.commit_message(&message, CommitMode::Async)?;
        }
    }
}
