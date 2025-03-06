use std::collections::HashMap;
use std::time::Duration;

use crate::hook::HookInitializer;
use crate::storage::MultipoolStorage;
use alloy::primitives::{Address, U256};
use alloy::rpc::types::Log;
use anyhow::{anyhow, Context};
use multipool::expiry::{EmptyTimeExtractor, MayBeExpired};
use multipool_types::kafka::KafkaTopics;
use rdkafka::consumer::BaseConsumer;
use rdkafka::Message;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct PriceData {
    address: Address,
    prices: HashMap<Address, MayBeExpired<U256, EmptyTimeExtractor>>,
}

pub async fn into_fetching_task<HI: HookInitializer>(
    storage: &mut MultipoolStorage<HI>,
    consumer: BaseConsumer,
    interval: Duration,
) -> anyhow::Result<()> {
    loop {
        let last_seen_block = storage.get_last_seen_block()?.unwrap_or(0);

        match consumer.poll(Duration::from_secs(1)) {
            Some(Ok(message)) => match message.topic().into() {
                KafkaTopics::ChainEvents => {
                    let bytes = message
                        .payload()
                        .context(anyhow!("Received message with no payload"))?;
                    let logs: Vec<Log> = serde_json::from_value(serde_json::Value::Array(
                        serde_json::from_slice::<Value>(bytes)
                            .unwrap()
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.get("row_event").unwrap().to_owned())
                            .collect::<Vec<Value>>(),
                    ))
                    .unwrap();
                    storage
                        .apply_events(logs, last_seen_block + 1, None)
                        .await?;
                }
                KafkaTopics::MpPrices => {
                    let bytes = message
                        .payload()
                        .context(anyhow!("Received message with no payload"))?;
                    let data = serde_json::from_slice::<PriceData>(bytes).unwrap();
                    storage.apply_prices(data.address, data.prices).await?;
                }
            },
            Some(Err(e)) => println!("KafkaConsumerError: {}", e),
            None => {}
        }
        tokio::time::sleep(interval).await;
    }
}
