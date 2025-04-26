use std::fs::{self, File};

use arwave_client::{Rpc, Signer, Transaction, Uploader};
use reqwest::Client;

#[tokio::main]
async fn main() {
    let signer = Signer::from_file("./wallet.json").unwrap();
    let big_data = fs::read("./arwave_client/BigImage.png").unwrap();
    let rpc = Rpc {
        url: "https://arweave.net:443".to_string(),
        client: Client::new(),
    };
    let mut tx = Transaction::builder(rpc.clone())
        .data(big_data)
        .build()
        .await
        .unwrap();
    tx.sign(signer).unwrap();
    let chunk = tx.get_chunk(3, &tx.data).unwrap();
    // println!("root {:?}", chunk.data_root);
    // println!("path {:?}", chunk.data_path);
    // println!("size {:?}", chunk.data_size);
    // println!("offset {:?}", chunk.offset);
    let mut uploader = Uploader::new(rpc, tx);
    uploader.upload_chunks().await.unwrap();
}
