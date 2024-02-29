use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use futures::{Future, FutureExt, TryFutureExt};
use tokio::fs;

use super::{ir::MultipoolStorageIR, MultipoolStorage};

#[derive(Debug, Clone)]
pub struct Ledger {
    folder_path: PathBuf,
}

impl Ledger {
    pub async fn new(folder_path: PathBuf) -> Result<Self> {
        let instance = Self {
            folder_path: folder_path.clone(),
        };
        if !fs::try_exists(&folder_path.join("store")).await? {
            instance.write(MultipoolStorageIR::default())?.await?;
        }
        Ok(instance)
    }

    pub fn read(&self) -> impl Future<Output = Result<MultipoolStorageIR>> {
        fs::read(self.folder_path.join("store")).map(|v| match v {
            Ok(v) => MultipoolStorageIR::try_unpack(v.as_slice()),
            Err(e) => Err(e.into()),
        })
    }

    pub fn write(&self, data: MultipoolStorageIR) -> Result<impl Future<Output = Result<()>>> {
        Ok(fs::write(self.folder_path.join("store"), data.try_pack()?).map_err(Into::into))
    }

    pub fn spawn_syncing_task(
        self,
        storage: MultipoolStorage,
        sync_interval: u64,
    ) -> impl Future<Output = Result<()>> {
        async move {
            tokio::task::spawn(async move {
                loop {
                    let ir = storage.build_ir().await;
                    self.write(ir)?.await?;
                    tokio::time::sleep(Duration::from_millis(sync_interval)).await;
                }
            })
            .await?
        }
    }
}
