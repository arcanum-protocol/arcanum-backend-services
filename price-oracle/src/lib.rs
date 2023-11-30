use ethers::abi::{Bytes, Token};
use ethers::prelude::*;
use ethers::utils::hex::encode_prefixed;
use futures::future::{join, join_all};
use futures::FutureExt;
use primitive_types::{U128, U256};
use reqwest;
use serde::{Deserialize, Serialize};
use std::future::{self, ready, Future};
use std::ops::{Mul, Shr};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::watch;
use tokio::time::{sleep, Duration};

const KYBERSWAP_API_URL: &str = "https://aggregator-api.kyberswap.com/arbitrum/api/v1/routes";

abigen!(
    Multipool,
    r#"[
        function getPrice(address asset) external view returns (uint price)
        function getAsset(address asset) external view returns (uint quantity, uint128 share, uint128 collectedCashbacks)
    ]"#,
);

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MultipoolState {
    contract_address: Address,
    assets_addresses: Vec<Address>,
}

type Client = Arc<Provider<Http>>;
type Price = U128;

#[derive(Serialize, Deserialize, Debug)]
pub struct SignedSharePrice {
    address: Address,
    timestamp: String,
    value: String,
    signature: String,
}

// врде это оригинал
const GMX_03: &str = "0x1aeedd3727a6431b8f070c0afaa81cc74f273882";
const RDNT_03: &str = "0x446bf9748b4ea044dd759d9b9311c70491df8f29";
const GMX: &str = "0xfc5a1a6eb076a2c7ad06ed22c90d7e710e35ad0a";
const RDNT: &str = "0x3082cc23568ea640225c2467653db90e9250aaa0";

const ETH_NATIVE: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"; //0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
const AMOUNT_IN: &str = "5100000000000000";

//#[tokio::main]
//async fn main() -> Result<(), Box<dyn std::error::Error>> {
//    // let denominator = U256::from(2).pow(U256::from(96));
//
//    // let rpc_url = std::env::var("RPC_URL").expect("RPC_URL val should be set");
//    // let private_key = std::env::var("KEY").expect("KEY val should be set");
//
//    // let provider = Provider::<Http>::try_from(rpc_url).unwrap();
//    // let client = Arc::new(provider);
//
//    // let mut address: Address = GMX_03.parse().unwrap();
//    // let multipool = Multipool::new(address, client.clone());
//
//    //let slot0_gmx = contract_gmx.slot_0().call().await.unwrap().0;
//}

impl MultipoolState {
    // fn get_price(&self, client: Multipool<Provider<Http>>) -> impl Future<Output = Price> {
    //     join_all(self.assets_addresses.into_iter().map(|address| {
    //         join(
    //             client.get_price(address).call().map(|val| val.unwrap()),
    //             client.get_asset(address).call().map(|val| val.unwrap().0),
    //         )
    //     }))
    //     .map(|values| {
    //         values
    //             .into_iter()
    //             .map(|(price, quantity)| quantity.mul(price).shr(96).as_u128())
    //             .sum::<u128>()
    //             .into()
    //     })
    // }

    pub fn from_address(contract_address: String) -> Self {
        Self {
            contract_address: contract_address.parse().unwrap(),
            assets_addresses: Default::default(),
        }
    }

    pub async fn sign<S: ethers::signers::Signer + 'static>(
        &self,
        price: Price,
        signer: &S,
    ) -> SignedSharePrice {
        let current_ts: U128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .into();
        let msg = abi::encode_packed(&[
            Token::Address(self.contract_address),
            Token::Uint(U256([
                current_ts.0[0],
                current_ts.0[1],
                price.0[0],
                price.0[1],
            ])),
        ])
        .unwrap();
        let msg = ethers::core::utils::hash_message(msg);
        signer
            .sign_message(msg)
            .map(move |signature| SignedSharePrice {
                address: self.contract_address,
                timestamp: current_ts.as_u128().to_string(),
                value: price.as_u128().to_string(),
                signature: encode_prefixed(
                    signature.expect("Signing should be successful").to_vec(),
                ),
            })
            .await
    }
}
