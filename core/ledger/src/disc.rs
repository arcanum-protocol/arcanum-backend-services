use std::path::PathBuf;

use anyhow::{anyhow, Result};
use futures::{Future, FutureExt, TryFutureExt};
use tokio::fs;

use crate::Ledger;

use crate::ir::MultipoolStorageIR;

#[derive(Debug, Clone)]
pub struct DiscLedger {
    path: PathBuf,
}

impl Ledger for DiscLedger {
    fn read(&self) -> impl Future<Output = Result<MultipoolStorageIR>> {
        fs::read(&self.path).map(|v| match v {
            Ok(v) => MultipoolStorageIR::try_unpack(v.as_slice()),
            Err(e) => Err(e.into()),
        })
    }

    fn write(&self, data: MultipoolStorageIR) -> Result<impl Future<Output = Result<()>>> {
        Ok(fs::write(&self.path, data.try_pack()?).map_err(Into::into))
    }
}

impl DiscLedger {
    /// Takes ledger at path and checks if it exist
    pub async fn at(path: PathBuf) -> Result<Self> {
        let instance = Self { path: path.clone() };
        if !fs::try_exists(&path).await? {
            Err(anyhow!("Ledger does not exist"))
        } else {
            Ok(instance)
        }
    }

    /// Initialises ledger at path
    pub async fn new(path: PathBuf) -> Result<Self> {
        let instance = Self { path: path.clone() };
        if !fs::try_exists(path).await? {
            instance.write(MultipoolStorageIR::default())?.await?;
            Ok(instance)
        } else {
            Err(anyhow!("Ledger already exists"))
        }
    }
}
