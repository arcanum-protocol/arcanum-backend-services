use anyhow::Result;
use futures::{Future, TryFutureExt};
use tokio::fs;

pub struct Ledger {
    path: String,
}

impl Ledger {
    pub fn read(&mut self) -> impl Future<Output = Result<Vec<u8>>> + '_ {
        fs::read(&self.path).map_err(Into::into)
    }

    pub fn write(&mut self, data: Vec<u8>) -> impl Future<Output = Result<()>> + '_ {
        fs::write(&self.path, data).map_err(Into::into)
    }
}
