mod embedded;
mod kafka;
mod pg;

pub use embedded::{EmbededProcessor, EmptyHookInitialiser};
pub use kafka::KafkaEventProcessor;
pub use pg::PgEventProcessor;
