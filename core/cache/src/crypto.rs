use ethers::abi::Token;
use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::prelude::*;
use ethers::utils::{self, hex};
use primitive_types::{U128, U256};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignedSharePrice {
    pub contract_address: Address,
    pub timestamp: String,
    pub share_price: String,
    pub signature: String,
}

pub fn sign(
    contract_address: Address,
    price: U256,
    chain_id: u128,
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
            Token::Uint(price),
            Token::Uint(U256::from(chain_id)),
        ]),
    ])
    .unwrap();

    let msg = utils::hash_message(utils::keccak256(msg));
    signer
        .sign_hash(msg)
        .map(move |signature| SignedSharePrice {
            contract_address,
            timestamp: current_ts.as_u128().to_string(),
            share_price: price.as_u128().to_string(),
            signature: hex::encode_prefixed(signature.to_vec()),
        })
        .unwrap()
}

#[cfg(test)]
pub mod test {

    use super::*;

    #[test]
    fn sign_data() {
        println!(
            "{:#?}",
            sign(
                "0x2e234DAe75C793f67A35089C9d99245E1C58470b"
                    .parse()
                    .unwrap(),
                U256::from(7922816251426433759354395033u128),
                31337,
                &"0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                    .parse()
                    .unwrap(),
            )
        )
    }
}
