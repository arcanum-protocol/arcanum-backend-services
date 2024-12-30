use alloy::{
    dyn_abi::DynSolValue,
    hex,
    primitives::{keccak256, Address, U128, U256},
    signers::{local::PrivateKeySigner, SignerSync},
};
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
    signer: &PrivateKeySigner,
) -> anyhow::Result<SignedSharePrice> {
    let current_ts = U128::from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    );

    let msg = DynSolValue::Tuple(vec![
        DynSolValue::Address(contract_address),
        DynSolValue::Tuple(vec![
            DynSolValue::Uint(U256::from(current_ts), 256),
            DynSolValue::Uint(price, 256),
            DynSolValue::Uint(U256::from(chain_id), 256),
        ]),
    ])
    .abi_encode();

    let msg = keccak256(msg);
    let signature = signer.sign_hash_sync(&msg)?;
    Ok(SignedSharePrice {
        contract_address,
        timestamp: current_ts.to_string(),
        share_price: price.to_string(),
        signature: hex::encode_prefixed(signature.as_bytes()),
    })
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
