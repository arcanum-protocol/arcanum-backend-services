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
    println!("{}", String::from_utf8(out.stderr).unwrap());

    let rpc = RpcRobber::from_anvil_mock(
        anvil.endpoint(),
        anvil.chain_id(),
        Some(
            "0x4368d13BD0D8B42062D4655Ca04607E60BB73F7b"
                .parse()
                .unwrap(),
        ),
    );

    let ledger: MockLedger = MultipoolStorageIR::default()
        .add_pool(
            MultipoolWithMeta::fill(
                ExternalMultipool {
                    contract_address: "0xb9F6151B58C127145FCB4Fd720dCAd3C10113638"
                        .parse()
                        .unwrap(),
                    assets: vec![
                        "0x3a822B7099a4D43099D26A2A5de655692F854fc0"
                            .parse()
                            .unwrap(),
                        "0x65177a42ce789f8111C879Bd53e96DEBED63c685"
                            .parse()
                            .unwrap(),
                        "0x9d9330483B898343Ef68aF2700076B5d610e210c"
                            .parse()
                            .unwrap(),
                        "0xFE8028Ba8EcB69c02fce2c47F3162E1150C5DDBa"
                            .parse()
                            .unwrap(),
                        "0xC09dD591C3C0C8f762378065649f98a02F512dA4"
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
            factory_address: "0xAfef20a6b3e05d6bc9b9541d9BF692E7914406F0"
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
        .set_hook(Some(()))
        .build()
        .await
        .expect("Failed to build storage");

    sleep(Duration::from_millis(500)).await;

    let address = "0xb9F6151B58C127145FCB4Fd720dCAd3C10113638"
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
