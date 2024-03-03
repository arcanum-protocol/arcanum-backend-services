use std::path::PathBuf;

use anyhow::Result;
use futures::{Future, FutureExt, TryFutureExt};
use tokio::fs;

use crate::Ledger;

use crate::ir::MultipoolStorageIR;

#[derive(Debug, Clone)]
pub struct DiscLedger {
    folder_path: PathBuf,
}

impl Ledger for DiscLedger {
    fn read(&self) -> impl Future<Output = Result<MultipoolStorageIR>> {
        fs::read(self.folder_path.join("store")).map(|v| match v {
            Ok(v) => MultipoolStorageIR::try_unpack(v.as_slice()),
            Err(e) => Err(e.into()),
        })
    }

    fn write(&self, data: MultipoolStorageIR) -> Result<impl Future<Output = Result<()>>> {
        Ok(fs::write(self.folder_path.join("store"), data.try_pack()?).map_err(Into::into))
    }
}

impl DiscLedger {
    pub async fn new(folder_path: PathBuf) -> Result<Self> {
        let instance = Self {
            folder_path: folder_path.clone(),
        };
        if !fs::try_exists(&folder_path.join("store")).await? {
            instance.write(MultipoolStorageIR::default())?.await?;
        }
        Ok(instance)
    }
}
