use alloy::{
    eips::BlockNumberOrTag, primitives::Address, providers::Provider, transports::BoxTransport,
};
use futures::StreamExt;

use crate::contracts;

pub struct MultipoolIndexer<T, P> {
    contract_instance: contracts::Multipool::MultipoolInstance<T, P>,
    from_block: BlockNumberOrTag,
}

impl<P: Provider> MultipoolIndexer<BoxTransport, P> {
    pub fn new(contract_address: Address, provider: P, from_block: BlockNumberOrTag) -> Self {
        Self {
            contract_instance: contracts::Multipool::new(contract_address, provider),
            from_block,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let mut asset_change_filter = self
            .contract_instance
            .AssetChange_filter()
            .from_block(self.from_block)
            .watch()
            .await?
            .into_stream();
        let mut target_share_change_filter = self
            .contract_instance
            .TargetShareChange_filter()
            .from_block(self.from_block)
            .watch()
            .await?
            .into_stream();

        loop {
            tokio::select! {
                event = asset_change_filter.next() => {
                    // TODO Handle the event
                }
                event = target_share_change_filter.next() => {
                    // TODO Handle the event
                }
            }
        }

        Ok(())
    }
}
