use anyhow::Result;
use futures::Future;

use crate::Ledger;

use crate::ir::MultipoolStorageIR;

#[derive(Debug, Clone)]
pub struct MockLedger {
    ir: MultipoolStorageIR,
}

impl Ledger for MockLedger {
    fn read(&self) -> impl Future<Output = Result<MultipoolStorageIR>> {
        futures::future::ready(Ok(self.ir.clone()))
    }

    fn write(&self, _data: MultipoolStorageIR) -> Result<impl Future<Output = Result<()>>> {
        Ok(futures::future::ready(Ok(())))
    }
}

impl From<MultipoolStorageIR> for MockLedger {
    fn from(value: MultipoolStorageIR) -> Self {
        Self { ir: value }
    }
}
