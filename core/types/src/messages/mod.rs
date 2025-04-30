use crate::expiry::{EmptyTimeExtractor, MayBeExpired};
use alloy::{
    primitives::{Address, LogData, U256},
    providers::Provider,
};
use anyhow::Context;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Clone)]
pub enum KafkaTopics {
    ChainEvents(u64),
    MpPrices(u64),
}

impl ToString for KafkaTopics {
    fn to_string(&self) -> String {
        match self {
            KafkaTopics::ChainEvents(chain_id) => format!("chain-events-{chain_id}"),
            KafkaTopics::MpPrices(chain_id) => format!("mp-prices-{chain_id}"),
        }
    }
}

impl TryFrom<&str> for KafkaTopics {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value
            .rsplit_once('-')
            .map(
                |(topic, chain_id): (&str, &str)| -> Result<Self, Self::Error> {
                    let chain_id = chain_id.parse()?;
                    let parsed_topic = match topic {
                        "chain-events" => KafkaTopics::ChainEvents(chain_id),
                        "mp-prices" => KafkaTopics::MpPrices(chain_id),
                        _ => unimplemented!("Kafka topic is not valid"),
                    };
                    Ok(parsed_topic)
                },
            )
            .ok_or(anyhow::anyhow!("Invalid topic"))?
    }
}

#[derive(Serialize, Debug)]
pub struct Blocks(pub Vec<Block>);

impl Blocks {
    pub async fn parse_logs<P: Provider + Clone + 'static>(
        logs: &[alloy::rpc::types::Log],
        rpc: P,
    ) -> anyhow::Result<Self> {
        let mut blocks = Vec::new();
        let mut blocks_timestamps = std::collections::HashMap::<u64, u64>::new();

        for log in logs {
            let block_number = log.block_number.context("Block number is absent")?;
            let transaction_index = log
                .transaction_index
                .context("Transaction index is absent")?;
            match blocks
                .iter()
                .position(|block: &Block| block.number.eq(&block_number))
            {
                Some(index) => match blocks[index]
                    .transactions
                    .iter()
                    .position(|txn| txn.index.eq(&transaction_index))
                {
                    None => {
                        blocks[index].transactions.push(Transaction {
                            hash: log
                                .transaction_hash
                                .context("Transaction hash is absent")?
                                .into(),
                            index: log
                                .transaction_index
                                .context("Transaction index is absent")?,
                            events: vec![Event {
                                log: log.inner.clone(),
                                index: log.log_index.context("Log index is absent")?,
                            }],
                        });
                    }
                    Some(txn_index) => {
                        let position = blocks[index].transactions[txn_index]
                            .events
                            .iter()
                            .take_while(|t| t.index.gt(&(txn_index as u64)))
                            .count();
                        blocks[index].transactions[txn_index].events.insert(
                            position,
                            Event {
                                log: log.inner.clone(),
                                index: log.log_index.context("Log index is absent")?,
                            },
                        );
                    }
                },
                None => {
                    let timestamp =
                        match (blocks_timestamps.get(&block_number), log.block_timestamp) {
                            (Some(v), _) => *v,
                            (None, Some(ts)) => ts,
                            (None, None) => {
                                let timestamp = rpc
                                    .get_block_by_hash(
                                        log.block_hash.context("Block hash is absent")?,
                                    )
                                    .await?
                                    .map(|b| b.header.timestamp)
                                    .context("Block timestamp is absent")?;
                                blocks_timestamps.insert(block_number, timestamp);
                                timestamp
                            }
                        };

                    let position = blocks
                        .iter()
                        .take_while(|t| t.number.gt(&block_number))
                        .count();
                    blocks.insert(
                        position,
                        Block {
                            number: block_number,
                            hash: log.block_hash.context("Block hash is absent")?.into(),
                            timestamp,
                            transactions: vec![Transaction {
                                hash: log
                                    .transaction_hash
                                    .context("Transaction hash is absent")?
                                    .into(),
                                index: log
                                    .transaction_index
                                    .context("Transaction index is absent")?,
                                events: vec![Event {
                                    log: log.inner.clone(),
                                    index: log.log_index.context("Log index is absent")?,
                                }],
                            }],
                        },
                    );
                }
            }
        }
        Ok(Blocks(blocks.iter().rev().map(|v| v.to_owned()).collect()))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    #[serde(rename = "n")]
    pub number: u64,
    #[serde(rename = "h")]
    pub hash: [u8; 32],
    #[serde(rename = "t")]
    pub timestamp: u64,
    #[serde(rename = "tx")]
    pub transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    #[serde(rename = "h")]
    pub hash: [u8; 32],
    #[serde(rename = "i")]
    pub index: u64,
    #[serde(rename = "e")]
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    #[serde(rename = "l")]
    pub log: alloy::primitives::Log<LogData>,
    #[serde(rename = "i")]
    pub index: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PriceData {
    pub address: Address,
    pub prices: Vec<(Address, MayBeExpired<U256, EmptyTimeExtractor>)>,
}

pub trait MsgPack<'de>
where
    Self: Serialize + Deserialize<'de>,
{
    fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }

    fn unpack(buf: &[u8]) -> Self {
        let cur = Cursor::new(buf);
        let mut de = Deserializer::new(cur);
        Deserialize::deserialize(&mut de).unwrap()
    }
}

impl<'de, T: Serialize + Deserialize<'de>> MsgPack<'de> for T {}

#[cfg(test)]
pub mod tests {
    use std::future;

    use alloy::rpc::client::RpcClient;
    use tokio::task::futures;

    use super::*;

    #[tokio::test]
    async fn check_block() {
        let logs = [
            alloy::rpc::types::Log {
                inner: alloy::primitives::Log {
                    address: [0u8; 20].into(),
                    data: LogData::default(),
                },
                removed: false,
                transaction_hash: Some([0u8; 32].into()),
                transaction_index: Some(1),
                log_index: Some(1),
                block_hash: Some([0u8; 32].into()),
                block_number: Some(10),
                block_timestamp: Some(11),
            },
            alloy::rpc::types::Log {
                inner: alloy::primitives::Log {
                    address: [0u8; 20].into(),
                    data: LogData::default(),
                },
                removed: false,
                transaction_hash: Some([0u8; 32].into()),
                transaction_index: Some(1),
                log_index: Some(2),
                block_hash: Some([0u8; 32].into()),
                block_number: Some(10),
                block_timestamp: Some(11),
            },
            alloy::rpc::types::Log {
                inner: alloy::primitives::Log {
                    address: [0u8; 20].into(),
                    data: LogData::default(),
                },
                removed: false,
                transaction_hash: Some([0u8; 32].into()),
                transaction_index: Some(1),
                log_index: Some(2),
                block_hash: Some([0u8; 32].into()),
                block_number: Some(11),
                block_timestamp: Some(12),
            },
            alloy::rpc::types::Log {
                inner: alloy::primitives::Log {
                    address: [0u8; 20].into(),
                    data: LogData::default(),
                },
                removed: false,
                transaction_hash: Some([0u8; 32].into()),
                transaction_index: Some(2),
                log_index: Some(2),
                block_hash: Some([0u8; 32].into()),
                block_number: Some(10),
                block_timestamp: Some(11),
            },
        ];
        let r = Blocks::parse_logs(
            logs.as_slice(),
            alloy::providers::ProviderBuilder::default().on_http(
                "https://endpoints.omniatech.io/v1/bsc/testnet/public"
                    .parse()
                    .unwrap(),
            ),
        )
        .await
        .unwrap();
        println!("{r:#?}");
    }
}
