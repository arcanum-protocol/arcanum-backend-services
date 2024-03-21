use anyhow::{anyhow, bail};
use clap::{Parser, Subcommand};

use ethers::prelude::*;
use multipool_ledger::{ir::MultipoolStorageIR, DiscLedger, Ledger};
use multipool_storage::{
    ir_builder::{ExternalFactory, ExternalMultipool, MultipoolStorageIRBuilder},
    multipool_with_meta::MultipoolWithMeta,
};
use rpc_controller::RpcRobber;
use url::Url;

use std::{env, path::PathBuf};

fn default_ledger_path() -> PathBuf {
    let mut path = env::current_dir().unwrap();
    path.push("ledger/");
    path
}

fn default_rpc_path() -> PathBuf {
    let mut path = env::current_dir().unwrap();
    path.push("rpc.yaml");
    path
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Path to ledger storage
    #[arg(short, long, default_value=default_ledger_path().into_os_string())]
    ledger: PathBuf,

    #[arg(short, long, default_value_t = false)]
    init: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Bootstrap {
        #[arg(long)]
        source_url: Url,
    },
    AddPool {
        #[arg(long)]
        address: Address,
        #[arg(long, value_delimiter = ',')]
        assets: Vec<Address>,
        #[arg(short, long, default_value=default_rpc_path().into_os_string())]
        rpc_config: PathBuf,
    },
    RemovePool {
        #[arg(long)]
        address: Address,
    },
    AddFactory {
        #[arg(long)]
        address: Address,
        #[arg(long)]
        start_block: u64,
    },
    RemoveFactory {
        #[arg(long)]
        address: Address,
    },
    Clone {
        #[arg(short, long)]
        to: PathBuf,
        #[arg(short, long)]
        compress: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let ledger = if args.init {
        DiscLedger::new(args.ledger).await?
    } else {
        DiscLedger::at(args.ledger).await?
    };

    if let Some(sub_command) = args.command {
        match sub_command {
            Commands::Bootstrap { source_url } => {
                let ir: MultipoolStorageIR = reqwest::get(source_url).await?.json().await?;
                if !ir.pools.is_empty() || !ir.factories.is_empty() {
                    bail!(anyhow!("Can't bootstrap a non empty ledger"));
                }
                ledger.write(ir)?.await?;
            }
            Commands::AddPool {
                address,
                assets,
                rpc_config,
            } => {
                let rpc = RpcRobber::read(rpc_config);
                let ir = ledger.read().await?;
                let ir = ir
                    .add_pool(
                        MultipoolWithMeta::fill(
                            ExternalMultipool {
                                contract_address: address,
                                assets,
                            },
                            &rpc,
                        )
                        .await?,
                    )
                    .ok_or(anyhow!("Pool already exist"))?;
                ledger.write(ir)?.await?;
            }
            Commands::AddFactory {
                address,
                start_block,
            } => {
                let ir = ledger.read().await?;
                let ir = ir
                    .add_factory(ExternalFactory {
                        factory_address: address,
                        block_number: start_block,
                    })
                    .ok_or(anyhow!("Pool already exist"))?;
                ledger.write(ir)?.await?;
            }
            Commands::RemovePool { address: _ } => {
                todo!("Remove is not yet implemented");
            }
            Commands::RemoveFactory { address: _ } => {
                todo!("Remove is not yet implemented");
            }
            Commands::Clone { to: _, compress: _ } => {
                todo!("Clone is not yet implemented");
            }
        }
    }
    Ok(())
}
