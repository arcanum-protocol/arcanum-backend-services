use std::time::Duration;

use crate::{Rpc, Transaction};
use anyhow::Result;

pub struct Uploader {
    rpc: Rpc,
    tx: Transaction,
    data: Vec<u8>,
    last_chunk: usize,
}

impl Uploader {
    pub fn new(rpc: Rpc, tx: Transaction) -> Self {
        Self {
            rpc,
            data: tx.data.clone(),
            tx,
            last_chunk: 1,
        }
    }

    pub async fn upload_chunks(&mut self) -> Result<()> {
        if self.tx.chunks.as_ref().unwrap().chunks.len() > 2 {
            self.tx.data = Vec::new();
        }
        self.rpc.post_tx(&self.tx).await?;
        while let Ok(chunk) = self.tx.get_chunk(self.last_chunk, &self.data) {
            self.rpc.chunk(chunk).await?;
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        Ok(())
    }
}
