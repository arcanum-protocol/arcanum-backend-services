use crate::{
    rpc::Rpc,
    transaction::{Tag, Transaction},
};
use anyhow::{Context, Result};

#[derive(Default)]
pub struct TransactionBuilder {
    rpc: Rpc,
    format: Option<u8>,
    last_tx: Option<String>,
    owner: Option<String>,
    tags: Option<Vec<Tag>>,
    target: Option<String>,
    quantity: Option<String>,
    data: Option<Vec<u8>>,
    data_size: Option<String>,
    data_root: Option<String>,
    reward: Option<String>,
}

impl TransactionBuilder {
    pub fn new(rpc: Rpc) -> Self {
        Self {
            rpc,
            ..Default::default()
        }
    }

    pub fn format(mut self, format: u8) -> Self {
        self.format = Some(format);
        self
    }

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn quantity(mut self, quantity: String) -> Self {
        self.quantity = Some(quantity);
        self
    }

    pub fn target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    pub fn owner(mut self, owner: String) -> Self {
        self.owner = Some(owner);
        self
    }

    pub async fn build(self) -> Result<Transaction> {
        let last_tx = if let Some(tx) = self.last_tx {
            tx
        } else {
            self.rpc.anchor().await?
        };
        let data = self.data.context("No data supplied")?;
        let reward = if let Some(reward) = self.reward {
            reward
        } else {
            self.rpc.get_price(data.len(), &self.target).await?
        };
        let mut tx = Transaction {
            format: self.format.unwrap_or(2),
            id: String::new(),
            last_tx,
            owner: self.owner.unwrap_or_default(),
            tags: self.tags.unwrap_or_default(),
            target: self.target.unwrap_or_default(),
            quantity: self.quantity.unwrap_or("0".to_string()),
            data_size: data.len().to_string(),
            data,
            reward,
            ..Default::default()
        };
        tx.get_signature_data().unwrap();
        Ok(tx)
    }
}
