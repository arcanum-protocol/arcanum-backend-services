use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use async_process::Command;
use ethers::utils::Anvil;
use tokio::time::sleep;

use rpc_controller::RpcRobber;

use multipool_ledger::{ir::MultipoolStorageIR, MockLedger};

use crate::builder::MultipoolStorageBuilder;

use crate::ir_builder::{ExternalMultipool, MultipoolStorageIRBuilder};
use crate::multipool_with_meta::MultipoolWithMeta;

#[tokio::test]
async fn storage_happy_path() -> Result<()> {
    let anvil = Anvil::default().block_time(1u64).spawn();

    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../../arcanum-contracts");
    println!("{}", d.display());

    let out = Command::new("forge")
        .arg("script")
        .arg("--root")
        .arg(d)
        .arg("DeployTestEnv")
        .arg("--rpc-url")
        .arg(anvil.endpoint())
        .arg("--broadcast")
        .output()
        .await?;
    println!("{out:?}");

    let rpc = RpcRobber::from_anvil_mock(
        anvil.endpoint(),
        anvil.chain_id(),
        Some(
            "0x1dC8a38A078A17DFFeDCa3f576C45aE7309611EE"
                .parse()
                .unwrap(),
        ),
    );

    let ledger: MockLedger = MultipoolStorageIR::default()
        .add_pool(
            MultipoolWithMeta::fill(
                ExternalMultipool {
                    contract_address: "0x195ADA83492986766C34f3e97bC1CA5454Aa2D46"
                        .parse()
                        .unwrap(),
                    assets: vec![
                        "0x501E089d6343dd5f5afC7cf522F026C0Bf6aaBa2"
                            .parse()
                            .unwrap(),
                        "0xE08568d896e1F4bd589f0D62Cf1e5eC28eD03512"
                            .parse()
                            .unwrap(),
                        "0xeb3136343921DFB2771F64fd3F07153513F8e347"
                            .parse()
                            .unwrap(),
                        "0xe392c9E817B2237a5AA192228aEAEfDA2F2c035F"
                            .parse()
                            .unwrap(),
                        "0x882BCE1C2045657E167E2dc5e897635770Ea2582"
                            .parse()
                            .unwrap(),
                    ],
                },
                &rpc,
            )
            .await
            .unwrap(),
        )
        .into();

    let (storage, handle) = MultipoolStorageBuilder::default()
        .ledger(ledger)
        .rpc(rpc)
        .target_share_interval(100)
        .price_interval(100)
        .ledger_sync_interval(100)
        .quantity_interval(100)
        .build()
        .await
        .expect("Failed to build storage");

    sleep(Duration::from_millis(500)).await;

    let address = "0x195ADA83492986766C34f3e97bC1CA5454Aa2D46"
        .parse()
        .unwrap();
    let pool = storage.get_pool(&address).await;
    println!("{pool:?}");

    Ok(())
}
