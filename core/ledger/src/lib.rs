use anyhow::Result;
use futures::Future;
use ir::MultipoolStorageIR;

mod disc;
pub mod ir;
mod mock;

pub use disc::DiscLedger;
pub use mock::MockLedger;

pub trait Ledger {
    fn read(&self) -> impl Future<Output = Result<MultipoolStorageIR>> + Send;

    fn write(&self, data: MultipoolStorageIR) -> Result<impl Future<Output = Result<()>> + Send>;
}
