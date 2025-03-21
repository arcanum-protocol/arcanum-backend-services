use crate::expiry::{EmptyTimeExtractor, MayBeExpired};
use alloy::primitives::{Address, LogData, U256};
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

pub struct Blocks(pub Vec<Block>);

pub struct ParseError;

impl TryFrom<&[alloy::rpc::types::Log]> for Blocks {
    type Error = ParseError;

    fn try_from(value: &[alloy::rpc::types::Log]) -> Result<Self, Self::Error> {
        let mut blocks = Vec::new();
        for log in value {
            let block_number = log.block_number.ok_or(ParseError)?;
            let transaction_index = log.transaction_index.ok_or(ParseError)?;
            match blocks
                .iter()
                .rev()
                .position(|block: &Block| block.number.eq(&block_number))
            {
                Some(index) => match blocks[index]
                    .transactions
                    .iter()
                    .rev()
                    .position(|txn| txn.index.eq(&transaction_index))
                {
                    None => {
                        blocks[index].transactions.push(Transaction {
                            hash: log.transaction_hash.ok_or(ParseError)?.into(),
                            index: log.transaction_index.ok_or(ParseError)?,
                            events: vec![Event {
                                log: log.inner.clone(),
                                index: log.log_index.ok_or(ParseError)?,
                            }],
                        });
                    }
                    Some(txn_index) => {
                        let position = blocks[index]
                            .transactions
                            .iter()
                            .rev()
                            .take_while(|t| t.index.gt(&(txn_index as u64)))
                            .count();
                        blocks[index].transactions[txn_index].events.insert(
                            position,
                            Event {
                                log: log.inner.clone(),
                                index: log.log_index.ok_or(ParseError)?,
                            },
                        );
                    }
                },
                None => {
                    let position = blocks
                        .iter()
                        .rev()
                        .take_while(|t| t.number.gt(&block_number))
                        .count();
                    blocks.insert(
                        position,
                        Block {
                            number: block_number,
                            hash: log.block_hash.ok_or(ParseError)?.into(),
                            timestamp: log.block_timestamp.ok_or(ParseError)?,
                            transactions: vec![Transaction {
                                hash: log.transaction_hash.ok_or(ParseError)?.into(),
                                index: log.transaction_index.ok_or(ParseError)?,
                                events: vec![Event {
                                    log: log.inner.clone(),
                                    index: log.log_index.ok_or(ParseError)?,
                                }],
                            }],
                        },
                    );
                }
            }
        }
        Ok(Blocks(blocks))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub number: u64,
    pub hash: [u8; 32],
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub hash: [u8; 32],
    pub index: u64,
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub log: alloy::primitives::Log<LogData>,
    pub index: u64,
}

#[derive(Serialize, Deserialize)]
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
