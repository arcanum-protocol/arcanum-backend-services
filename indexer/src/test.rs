use std::time::Duration;

use alloy::{
    network::EthereumWallet, node_bindings::Anvil, primitives::aliases::U96,
    providers::ProviderBuilder, signers::local::PrivateKeySigner, sol_types::SolCall,
};
use anyhow::Result;
use indexer1::Indexer;
use multipool::Multipool;
use multipool_storage::storage::MultipoolStorage;
use multipool_types::MultipoolFactory::{self, MultipoolCreationParams};

use crate::{EmbededProcessor, EmptyHookInitialiser};

#[sqlx::test]
async fn happy_path(pool: sqlx::SqlitePool) -> Result<()> {
    let anvil = Anvil::new().block_time_f64(0.1).try_spawn()?;

    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let owner_address = signer.address();

    // Create a provider.
    let ws = alloy::providers::WsConnect::new(anvil.ws_endpoint());
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let mp_implementation = multipool_types::Multipool::deploy(provider.clone()).await?;

    let factory_implementation =
        multipool_types::MultipoolFactory::deploy(provider.clone()).await?;

    let factory = multipool_types::Proxy::deploy(
        provider.clone(),
        *factory_implementation.address(),
        MultipoolFactory::initializeCall {
            owner: owner_address,
            implementation: *mp_implementation.address(),
        }
        .abi_encode()
        .into(),
    )
    .await?;

    let factory = multipool_types::MultipoolFactory::new(*factory.address(), provider);

    let params = MultipoolCreationParams {
        name: "Token".into(),
        symbol: "Token".into(),
        initialSharePrice: U96::from(10000),
        deviationIncreaseFee: Default::default(),
        deviationLimit: Default::default(),
        feeToCashbackRatio: Default::default(),
        baseFee: Default::default(),
        managementFeeRecepient: Default::default(),
        managementFee: Default::default(),
        oracleAddress: Default::default(),
        assetAddresses: Default::default(),
        strategyManager: Default::default(),
        priceData: Default::default(),
        targetShares: Default::default(),
    };

    factory
        .createMultipool(params)
        .send()
        .await?
        .watch()
        .await?;

    let db = sled::open("test_db")?;
    let storage = MultipoolStorage::init(
        db,
        EmptyHookInitialiser,
        *factory.address(),
    )
    .await?;

    Indexer::builder()
        .sqlite_storage(pool)
        .http_rpc_url(anvil.endpoint_url())
        .ws_rpc_url(anvil.ws_endpoint_url())
        .fetch_interval(Duration::from_millis(100))
        .filter(Multipool::filter())
        .set_processor(EmbededProcessor::from_storage(storage))
        .build()
        .await?
        .run()
        .await?;

    Ok(())
}
