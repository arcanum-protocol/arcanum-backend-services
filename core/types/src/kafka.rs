#[derive(Clone)]
pub enum KafkaTopics {
    ChainEvents,
    MpPrices,
}

impl AsRef<str> for KafkaTopics {
    fn as_ref(&self) -> &str {
        match self {
            KafkaTopics::ChainEvents => "chain_events",
            KafkaTopics::MpPrices => "mp_prices",
        }
    }
}


impl From<&str> for KafkaTopics {
    fn from(value: &str) -> Self {
        match value {
            "chain_events" => KafkaTopics::ChainEvents,
            "mp_prices" => KafkaTopics::MpPrices,
            _ => unimplemented!(),
        }
    }
}
