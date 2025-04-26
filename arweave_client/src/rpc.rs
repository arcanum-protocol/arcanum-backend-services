use anyhow::Result;
use reqwest::Client;

use crate::transaction::{GetChunk, Transaction};

#[derive(Default, Clone)]
pub struct Rpc {
    pub url: String,
    pub client: Client,
}

impl Rpc {
    pub async fn anchor(&self) -> Result<String> {
        let res = self
            .client
            .get(format!("{}/tx_anchor", self.url))
            .send()
            .await?;
        Ok(res.text().await?)
    }

    pub async fn get_price(&self, size: usize, target: &Option<String>) -> Result<String> {
        let endpoint = if let Some(target) = target {
            format!("price/{}/{}", size, target)
        } else {
            format!("price/{}", size)
        };
        let res = self
            .client
            .get(format!("{}/{}", self.url, endpoint))
            .send()
            .await?;
        let val = res.text().await?;
        Ok(val)
    }

    pub async fn post_tx(&self, tx: &Transaction) -> Result<()> {
        // println!("{:?}", tx);
        let res = self
            .client
            .post(format!("{}/tx", self.url))
            .header("Content-type", "Application/json")
            .header("Accept", "application/json, text/plain, */*")
            .json(&tx)
            .send()
            .await?;
        println!("{:?}", res.text().await?);
        Ok(())
    }

    pub async fn chunk(&self, chunk: GetChunk) -> Result<()> {
        // println!("{:?}", chunk);
        let res = self
            .client
            .post(format!("{}/chunk", self.url))
            .header("Content-type", "Application/json")
            .header("Accept", "application/json, text/plain, */*")
            .json(&chunk)
            .send()
            .await?;
        println!("{:?}", res.text().await?);
        Ok(())
    }
}
