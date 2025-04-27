use std::fs::{self, File};

use arweave_client::{Rpc, Signer, Tag, Transaction, Uploader};
use reqwest::Client;

#[tokio::main]
async fn main() {
    let signer = Signer::from_file("./wallet.json").unwrap();
    // let big_data = fs::read("./arweave_client/BigImage.png").unwrap();
    let rpc = Rpc {
        url: "https://arweave.net:443".to_string(),
        client: Client::new(),
    };
    let mut tx = Transaction::builder(rpc.clone())
        .tags(vec![Tag {
            name: "Content-Type".to_string(),
            value: "MpData".to_string(),
        }])
        .data(b"Some message".to_vec())
        .build()
        .await
        .unwrap();
    tx.sign(signer.into()).unwrap();
    // let chunk = tx.get_chunk(3, &tx.data).unwrap();
    // println!("root {:?}", chunk.data_root);
    // println!("path {:?}", chunk.data_path);
    // println!("size {:?}", chunk.data_size);
    println!("tx {:?}", tx);
    // let mut uploader = Uploader::new(rpc, tx);
    // uploader.upload_chunks().await.unwrap();
}
