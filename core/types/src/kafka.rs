use alloy::rpc::types::Log;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub enum KafkaTopics {
    ChainEvents,
    MpPrices,
}

impl AsRef<str> for KafkaTopics {
    fn as_ref(&self) -> &str {
        match self {
            KafkaTopics::ChainEvents => "chain-events",
            KafkaTopics::MpPrices => "mp-prices",
        }
    }
}

impl From<&str> for KafkaTopics {
    fn from(value: &str) -> Self {
        match value {
            "chain-events" => KafkaTopics::ChainEvents,
            "mp-prices" => KafkaTopics::MpPrices,
            _ => unimplemented!("Kafka topic is not valid"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChainBlock {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_timestamp: Option<u64>,
    pub events: Vec<ChainEvent>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChainEvent {
    pub parsed_event: Option<Value>,
    pub transaction_hash: String,
    pub emitter_address: String,
    pub event_index: Option<u64>,
    pub row_event: Log,
}
