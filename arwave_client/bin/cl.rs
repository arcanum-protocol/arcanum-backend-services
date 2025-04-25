use std::fs;

use arwave_client::{rpc::Rpc, transaction::Transaction, wallet::Wallet};
use reqwest::Client;

#[tokio::main]
async fn main() {
    let text = fs::read_to_string("./wallet.json").unwrap();
    let wallet: Wallet = serde_json::from_str(&text).unwrap();
    let rpc = Rpc {
        url: "https://arweave.net".to_string(),
        client: Client::new(),
    };
    let mut tx = Transaction::builder(rpc.clone())
        .data(
            "Name: Sepolia MP, Symbol: Sep, Decimals: 18, Address: 0x..."
                .as_bytes()
                .to_vec(),
        )
        .build()
        .await
        .unwrap();
    tx.sign(wallet).unwrap();
    rpc.post_tx(tx).await.unwrap();
}
