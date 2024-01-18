use ethers::core::types::Filter;
use ethers::prelude::*;
use ethers::types::Log;
use futures::Future;
use futures::FutureExt;
use log::info;
use primitive_types::U256;
use std::sync::Arc;
use std::time::Duration;

use super::Share;
use super::{Price, Quantity};

static WAITING_TIME: u64 = 2;

abigen!(
    MultipoolContract,
    r#"[
        function getPrice(address asset) external view returns (uint price)
        function totalSupply() external view returns (uint totalSupply)
        function totalTargetShares() external view returns (uint targetShares)
        function getAsset(address asset) external view returns (uint quantity, uint128 share, uint128 collectedCashbacks)
    ]"#,
);

#[derive(Debug, Clone)]
pub struct QuantityUpdate {
    pub address: Address,
    pub quantity: Quantity,
}

pub struct MultipoolContractInterface {
    multipool: MultipoolContract<Provider<Http>>,
}

pub struct Asset {
    pub share: Share,
    pub quantity: Quantity,
    pub cashback: Quantity,
}

impl MultipoolContractInterface {
    pub fn new(contract_address: Address, provider: Arc<Provider<Http>>) -> Self {
        Self {
            multipool: MultipoolContract::new(contract_address, provider),
        }
    }

    pub fn get_asset_price(
        &self,
        asset_address: Address,
    ) -> impl Future<Output = Result<Price, ContractError<Provider<Http>>>> {
        let multipool = self.multipool.clone();
        async move {
            let f = multipool.get_price(asset_address);
            let fut = f.call().map(|v| v.map(|v| U256(v.0)));
            info!("start calling get price");
            tokio::select! {
                val = fut => {
                    info!("calling get price");
                    val
                }
                _ = tokio::time::sleep(Duration::from_secs(3)) => {
                    info!("Imeout for block to exceeded");
                    std::process::exit(0x0500);
                }
            }
        }
    }

    pub fn get_asset(
        &self,
        asset_address: Address,
    ) -> impl Future<Output = Result<Asset, ContractError<Provider<Http>>>> {
        let multipool = self.multipool.clone();
        async move {
            let f = multipool.get_asset(asset_address);
            let fut = f.call().map(|v| {
                v.map(|v| Asset {
                    quantity: v.0,
                    share: v.1.into(),
                    cashback: v.2.into(),
                })
            });
            info!("start calling get asset");
            tokio::select! {
                val = fut => {
                    info!("calling get asset");
                    val
                }
                _ = tokio::time::sleep(Duration::from_secs(WAITING_TIME)) => {
                    info!("Imeout for block to exceeded");
                    std::process::exit(0x0500);
                }
            }
        }
    }

    pub fn get_total_supply(
        &self,
    ) -> impl Future<Output = Result<Quantity, ContractError<Provider<Http>>>> {
        let multipool = self.multipool.clone();
        async move {
            let f = multipool.total_supply();
            info!("start calling total supply");
            tokio::select! {
                val = f.call() => {
                    info!("calling total supply");
                    val
                }
                _ = tokio::time::sleep(Duration::from_secs(3)) => {
                    info!("Imeout for block to exceeded");
                    std::process::exit(0x0500);
                }
            }
        }
    }

    pub fn get_total_shares(
        &self,
    ) -> impl Future<Output = Result<Share, ContractError<Provider<Http>>>> {
        let multipool = self.multipool.clone();
        async move {
            let f = multipool.total_target_shares();
            info!("start calling total shares");
            tokio::select! {
                val = f.call() => {
                    info!("calling total shares");
                    val
                }
                _ = tokio::time::sleep(Duration::from_secs(WAITING_TIME)) => {
                    info!("Imeout for block to exceeded");
                    std::process::exit(0x0500);
                }
            }
        }
    }
}

impl QuantityUpdate {
    pub fn get_event_updates(
        address: Address,
        block_from: U64,
        step_limit: U64,
        client: Arc<Provider<Http>>,
    ) -> impl Future<Output = Result<impl IntoIterator<Item = QuantityUpdate>, ProviderError>> {
        async move {
            let block_to = client.get_block_number().map(|result| {
                result.map(|current_block| {
                    if current_block - block_from > step_limit {
                        block_from + step_limit
                    } else {
                        current_block
                    }
                })
            });
            info!("start calling block to");
            let block_to = tokio::select! {
                val = block_to => {
                    info!("calling block to");
                    val.unwrap_or_else(|error| {
                        info!("Should successfully fetch block to: {:#?}", error);
                        std::process::exit(0x0500);
                    })
                }
                _ = tokio::time::sleep(Duration::from_secs(WAITING_TIME)) => {
                    info!("Imeout for block to exceeded");
                    std::process::exit(0x0500);
                }
            };

            let filter = Filter::new()
                .address(address)
                .event("AssetChange(address,uint256,uint128)")
                .from_block(block_from)
                .to_block(block_to);
            let logs = client.get_logs(&filter);

            info!("start calling get logs");
            let val = tokio::select! {
                val = logs => {
                    info!("calling get logs");
                    val.map(|logs|logs.into_iter().map(Into::into))
                }
                _ = tokio::time::sleep(Duration::from_secs(WAITING_TIME)) => {
                    info!("Get logs for block to exceeded");
                    std::process::exit(0x0500);
                }
            };
            val
        }
    }
}

impl From<Log> for QuantityUpdate {
    fn from(log: Log) -> Self {
        assert!(
            log.topics[0]
                == "0xb61cae05ab66ffbfeccab110d47efe9c7dc7af5b59b5030f3d8f60df191a4643"
                    .parse()
                    .unwrap()
        );
        Self {
            address: Address::from(log.topics[1]),
            quantity: Quantity::from_big_endian(&log.data[0..32]),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetching_logs() {
        let rpc_url = "http://81.163.22.190:8545";
        let provider = Provider::<Http>::try_from(rpc_url).unwrap();
        let client = Arc::new(provider);
        let res = QuantityUpdate::get_event_updates(
            "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
                .parse()
                .unwrap(),
            0.into(),
            100000000.into(),
            client,
        )
        .await;
        let val = res.unwrap().into_iter().collect::<Vec<_>>();

        println!("res {:#?}", val);
    }
}
