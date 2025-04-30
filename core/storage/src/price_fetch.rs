use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{address, Address, U256};
use alloy::providers::{CallItemBuilder, Provider, MULTICALL3_ADDRESS};
use itertools::Itertools;
use alloy::providers::MulticallBuilder;

pub async fn get_asset_prices<P: Provider + Clone + 'static>(
    mp: Address,
    assets: Vec<Address>,
    chunk_size: usize,
    provider: &P,
) -> anyhow::Result<Vec<(Address, U256)>> {
    let mp = multipool_types::Multipool::new(mp, provider);

    let mut prices = Vec::new();
    let chunked_assets = assets
        .iter()
        .chunks(chunk_size)
        .into_iter()
        .map(|chunk| chunk.into_iter().collect_vec())
        .collect_vec();
    for chunk in chunked_assets {
        let calls: Vec<_> = chunk.into_iter().map(|address| mp.getPrice(*address)).collect();

        let result = MulticallBuilder::new_dynamic(provider)
            .extend(calls)
            .aggregate3()
            .await?
            .into_iter()
            .map(|p| p.unwrap());
        prices.extend(result);
    }
    // todo!();
    Ok(assets.into_iter().zip(prices.into_iter()).collect())
}
