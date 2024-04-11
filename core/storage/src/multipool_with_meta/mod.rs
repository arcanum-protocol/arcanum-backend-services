use multipool::{expiry::StdTimeExtractor, Multipool};
use multipool_ledger::ir::Time;

pub mod multipool_events;
pub mod multipool_prices;

#[derive(Debug, Clone)]
pub struct MultipoolWithMeta {
    pub multipool: Multipool<StdTimeExtractor>,
    pub quantity_time: Time,
    pub share_time: Time,
}
