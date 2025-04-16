use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, MULTICALL3_ADDRESS};
use std::ops::Shl;

pub async fn get_mps_prices<P: Provider + Clone + 'static>(
    mps: Vec<Address>,
    provider: &P,
) -> anyhow::Result<(Vec<(Address, U256)>, U256)> {
    let multipool_functions = multipool_types::Multipool::abi::functions();
    let get_price_func = &multipool_functions.get("getSharePricePart").unwrap()[0];
    let mut mc = alloy_multicall::Multicall::new(
        &provider,
        MULTICALL3_ADDRESS, // address!("cA11bde05977b3631167028862bE2a173976CA11"),
    );
    for mp in mps.iter() {
        mc.add_call(
            *mp,
            get_price_func,
            &[
                DynSolValue::Uint(U256::MAX, 256),
                DynSolValue::Uint(U256::ZERO, 256),
            ],
            true,
        );
    }

    mc.add_get_current_block_timestamp();
    let mut res = mc.call().await?;
    let ts = res.pop().unwrap().unwrap().as_uint().unwrap().0;
    let prices = res
        .into_iter()
        .map(|p| p.unwrap().as_uint().unwrap().0.shl(96));

    Ok((mps.into_iter().zip(prices).collect(), ts))
}
