use std::path::PathBuf;

use anyhow::{anyhow, Result};
use futures::{Future, FutureExt};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::Ledger;

use crate::ir::MultipoolStorageIR;

#[derive(Debug, Clone)]
pub struct DiscLedger {
    path: PathBuf,
}

impl Ledger for DiscLedger {
    fn read(&self) -> impl Future<Output = Result<MultipoolStorageIR>> {
        fs::read(self.path.as_path().join("state").clone()).map(|v| match v {
            Ok(v) => MultipoolStorageIR::try_unpack(v.as_slice()),
            Err(e) => Err(e.into()),
        })
    }

    fn write(&self, data: MultipoolStorageIR) -> Result<impl Future<Output = Result<()>>> {
        let tmp_path = self.path.join("tmp");
        let state_path = self.path.join("state");
        let packed_data = data.try_pack()?;
        Ok(async move {
            let mut file = fs::File::create(tmp_path.clone()).await?;
            file.write_all(&packed_data).await?;
            file.sync_all().await?;
            fs::rename(tmp_path, state_path).await?;
            Ok(())
        })
    }
}

impl DiscLedger {
    /// Takes ledger at path and checks if it exist
    pub async fn at(path: PathBuf) -> Result<Self> {
        let instance = Self { path: path.clone() };
        if !fs::try_exists(&path.join("state")).await? {
            Err(anyhow!("Ledger folder does not exist"))
        } else {
            Ok(instance)
        }
    }

    /// Initialises ledger at path
    pub async fn new(path: PathBuf) -> Result<Self> {
        let instance = Self { path: path.clone() };
        if !fs::try_exists(&path).await? {
            fs::create_dir(&path).await?;
        }
        if !fs::try_exists(path.join("state")).await? {
            instance.write(MultipoolStorageIR::default())?.await?;
            Ok(instance)
        } else {
            Err(anyhow!("Ledger already exists"))
        }
    }
}
