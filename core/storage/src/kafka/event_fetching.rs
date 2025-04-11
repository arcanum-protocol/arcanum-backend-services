use crate::hook::HookInitializer;
use crate::storage::MultipoolStorage;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use multipool_types::messages::{self, KafkaTopics, MsgPack, PriceData};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::Message;

pub async fn into_fetching_task<HI: HookInitializer>(
    storage: &mut MultipoolStorage<HI>,
    consumer: StreamConsumer,
) -> anyhow::Result<()> {
    loop {
        let mut stream = consumer.stream();
        // add better error handling
        while let Some(Ok(message)) = stream.next().await {
            match message.topic().try_into()? {
                KafkaTopics::ChainEvents(chain_id) => {
                    let bytes = message
                        .payload()
                        .context(anyhow!("Received message with no payload"))?;
                    let blocks = vec![messages::Block::unpack(bytes)];

                    storage
                        .create_multipools(&chain_id, blocks.as_slice().try_into()?)
                        .await?;

                    storage
                        .apply_events(&chain_id, blocks.as_slice().try_into()?)
                        .await?;
                }
                KafkaTopics::MpPrices(chain_id) => {
                    let bytes = message
                        .payload()
                        .context(anyhow!("Received message with no payload"))?;
                    let data = PriceData::unpack(bytes);
                    storage.apply_prices(data.address, data.prices).await?;
                }
            }
            consumer.commit_message(&message, CommitMode::Sync)?;
        }
    }
}
