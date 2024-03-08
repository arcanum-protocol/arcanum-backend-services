use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use async_process::Command;
use ethers::utils::Anvil;
use tokio::time::sleep;

use rpc_controller::RpcRobber;

use multipool_ledger::{ir::MultipoolStorageIR, MockLedger};

use crate::builder::MultipoolStorageBuilder;

use crate::ir_builder::{ExternalFactory, ExternalMultipool, MultipoolStorageIRBuilder};
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
        .arg(&d)
        .arg("DeployTestEnv")
        .arg("--rpc-url")
        .arg(anvil.endpoint())
        .arg("--broadcast")
        .output()
        .await?;
    println!("{}", String::from_utf8(out.stdout).unwrap());

    let rpc = RpcRobber::from_anvil_mock(
        anvil.endpoint(),
        anvil.chain_id(),
        Some(
            "0x340f0BCD1310306eD33eF881fEABB18d788D6328"
                .parse()
                .unwrap(),
        ),
    );

    let ledger: MockLedger = MultipoolStorageIR::default()
        .add_pool(
            MultipoolWithMeta::fill(
                ExternalMultipool {
                    contract_address: "0xA7Da2C3C2e2CCbF69dE3F9b089A7c4a6A74156Bf"
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
        .unwrap()
        .add_factory(ExternalFactory {
            factory_address: "0x6fab5332a5F677613C1Eba902d82B1BE15DE4D07"
                .parse()
                .unwrap(),
            block_number: 0,
        })
        .unwrap()
        .into();

    let storage = MultipoolStorageBuilder::default()
        .ledger(ledger)
        .rpc(rpc)
        .monitoring_interval(100)
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

    let out = Command::new("forge")
        .arg("script")
        .arg("--root")
        .arg(&d)
        .arg("DeployEtfWithFactory")
        .arg("--rpc-url")
        .arg(anvil.endpoint())
        .arg("--broadcast")
        .output()
        .await?;
    println!("{}", String::from_utf8(out.stdout).unwrap());

    sleep(Duration::from_millis(500)).await;

    let pools = storage.pools().await;
    println!("{pools:#?}");

    storage.abort_handles().await;

    Ok(())
}
