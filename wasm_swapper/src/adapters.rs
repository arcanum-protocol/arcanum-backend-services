use ethers::{
    prelude::*,
    providers::{Http, Provider},
};
use futures::{future::join_all, TryFutureExt};
use multipool::QuantityData;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, js_sys::Promise};

use crate::{contracts::multipool::MpAsset, MultipoolWasmStorage};

use anyhow::Result;

#[wasm_bindgen]
impl MultipoolWasmStorage {
    #[wasm_bindgen]
    pub async fn update_price(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let mp = inner.borrow_mut();
            let multipool = &mp.multipool;
            let contract_address = multipool.contract_address();
            let assets = mp.assets.clone();
            let provider = mp.provider.clone();
            drop(mp);
            match get_prices(&provider, contract_address, assets, 6).await {
                Ok(prices) => {
                    inner.borrow_mut().multipool.update_prices(&prices, false);
                    Ok(JsValue::UNDEFINED)
                }
                Err(e) => Ok(e.to_string().into()),
            }
        })
    }

    #[wasm_bindgen]
    pub async fn update_assets(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let mp = inner.borrow_mut();
            let multipool = &mp.multipool;
            let assets = mp.assets.clone();
            let contract_address = multipool.contract_address();
            let provider = mp.provider.clone();
            drop(mp);
            let total_supply = total_supply(provider.clone(), contract_address)
                .await
                .map_err(|e| JsValue::from(e.to_string()))?;
            match get_assets(&provider, contract_address, assets, 6).await {
                Ok(assets) => {
                    let mp = &mut inner.borrow_mut().multipool;
                    let mut quantities = assets
                        .iter()
                        .map(|(asset_address, quantity_data)| {
                            (
                                *asset_address,
                                QuantityData {
                                    cashback: quantity_data.collected_cashbacks.into(),
                                    quantity: quantity_data.quantity,
                                },
                            )
                        })
                        .collect::<Vec<_>>();
                    quantities.push((
                        contract_address,
                        QuantityData {
                            quantity: total_supply,
                            cashback: U256::zero(),
                        },
                    ));

                    mp.update_quantities(&quantities, false);

                    mp.update_shares(
                        &assets
                            .iter()
                            .map(|(asset_address, quantity_data)| {
                                (*asset_address, quantity_data.target_share.into())
                            })
                            .collect::<Vec<_>>(),
                        false,
                    );
                    Ok(JsValue::UNDEFINED)
                }
                Err(e) => Ok(e.to_string().into()),
            }
        })
    }
}

pub fn multipool_at(
    contract_address: Address,
    provider: Provider<Http>,
) -> crate::contracts::multipool::MultipoolContract<providers::Provider<providers::Http>> {
    crate::contracts::multipool::MultipoolContract::new(
        contract_address,
        std::sync::Arc::new(provider),
    )
}

pub async fn total_supply(rpc: Provider<Http>, contract_address: Address) -> Result<U256> {
    multipool_at(contract_address, rpc)
        .total_supply()
        .await
        .map_err(|e| e.into())
}

pub async fn get_prices(
    rpc: &Provider<Http>,
    contract_address: Address,
    assets: Vec<Address>,
    multicall_chunks: usize,
) -> Result<Vec<(Address, U256)>> {
    join_all(assets.chunks(multicall_chunks).map(|assets| {
        async {
            let mp = multipool_at(contract_address, rpc.clone());
            Multicall::new(rpc.clone(), None)
                .await
                .unwrap()
                .add_calls(true, assets.iter().map(|asset| mp.get_price(*asset)))
                .call_array()
                .await
                .map_err(Into::into)
        }
        .map_ok(|data| {
            assets
                .iter()
                .cloned()
                .zip(data)
                .collect::<Vec<(Address, U256)>>()
        })
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<Vec<(Address, U256)>>>>()
    .map(|data| data.into_iter().flatten().collect())
}

pub async fn get_assets(
    rpc: &Provider<Http>,
    contract_address: Address,
    assets: Vec<Address>,
    multicall_chunks: usize,
) -> Result<Vec<(Address, MpAsset)>> {
    join_all(assets.chunks(multicall_chunks).map(|assets| {
        async {
            let mp = multipool_at(contract_address, rpc.clone());
            Multicall::new(rpc.clone(), None)
                .await
                .unwrap()
                .add_calls(true, assets.iter().map(|asset| mp.get_asset(*asset)))
                .call_array()
                .await
                .map_err(Into::into)
        }
        .map_ok(|data| {
            assets
                .iter()
                .cloned()
                .zip(data)
                .collect::<Vec<(Address, MpAsset)>>()
        })
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<Vec<(Address, MpAsset)>>>>()
    .map(|data| data.into_iter().flatten().collect())
}
