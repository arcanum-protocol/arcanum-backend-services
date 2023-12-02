use ethers::abi::Token;
use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::prelude::*;
use ethers::utils::{self, hex};
use primitive_types::{U128, U256};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::multipool_storage::Price;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "camelCase")]
pub struct SignedSharePrice {
    this_address: Address,
    timestamp: String,
    value: String,
    signature: String,
}

pub fn sign(
    contract_address: Address,
    price: Price,
    signer: &ethers::signers::Wallet<SigningKey>,
) -> SignedSharePrice {
    let current_ts: U128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .into();
    let msg = abi::encode_packed(&[
        Token::Address(contract_address),
        Token::FixedArray(vec![
            Token::Uint(U256::from(current_ts)),
            Token::Uint(U256::from(price)),
        ]),
    ])
    .unwrap();

    let msg = utils::hash_message(utils::keccak256(msg));
    signer
        .sign_hash(msg)
        .map(move |signature| SignedSharePrice {
            this_address: contract_address,
            timestamp: current_ts.as_u128().to_string(),
            value: price.as_u128().to_string(),
            signature: hex::encode_prefixed(signature.to_vec()),
        })
        .unwrap()
}
