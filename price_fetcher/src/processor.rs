use alloy::primitives::{address, Address, U256};
use alloy::providers::bindings::IMulticall3::getCurrentBlockTimestampCall;
use alloy::providers::MulticallBuilder;
use alloy::providers::{CallItem, MulticallItem};
use alloy::providers::{Provider, MULTICALL3_ADDRESS};
use alloy::sol_types::SolCall;
use std::ops::Shl;

// TODO: consider remove this method in favor of gateway/price_fetcher with same functionality
pub async fn get_mps_prices<P: Provider + Clone + 'static>(
    mps: Vec<Address>,
    provider: &P,
) -> anyhow::Result<(Vec<(Address, U256)>, U256)> {
    let calls: Vec<_> = mps
        .iter()
        .map(|mp| {
            let mp = multipool_types::Multipool::new(*mp, &provider);
            let call = mp.getSharePricePart(U256::MAX, U256::MIN);
            let target = call.target();
            let input = call.input();
            CallItem::<getCurrentBlockTimestampCall>::new(target, input)
        })
        .collect();
    let get_ts = CallItem::<getCurrentBlockTimestampCall>::new(
        address!("cA11bde05977b3631167028862bE2a173976CA11"),
        getCurrentBlockTimestampCall {}.abi_encode().into(),
    );
    let mut result = MulticallBuilder::new_dynamic(provider)
        .extend_calls(calls)
        .add_call_dynamic(get_ts)
        .aggregate3()
        .await?;

    let ts = result.pop().unwrap().unwrap();
    let prices = result.into_iter().map(|p| p.unwrap().shl(96));

    Ok((mps.into_iter().zip(prices).collect(), ts))
}
