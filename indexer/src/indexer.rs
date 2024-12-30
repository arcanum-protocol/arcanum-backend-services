use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, U256, U64},
    providers::Provider,
    transports::BoxTransport,
};
use futures::StreamExt;

use crate::{
    contracts::{
        self, AssetChangeEvent, MultipoolFactory::MultipoolFactoryInstance, MultipoolSpawnedEvent,
        TargetShareChangeEvent,
    },
    raw_storage::RawEventStorage,
};

pub struct MultipoolIndexer<T, P, R: RawEventStorage> {
    contract_instance: contracts::Multipool::MultipoolInstance<T, P>,
    factory_contract_address: Address,
    chain_id: String,
    from_block: BlockNumberOrTag,
    raw_storage: R,
    provider: P,
    multipool_storage: crate::multipool_storage::MultipoolStorage,
}

impl<P: Provider + Clone + 'static, R: RawEventStorage + Clone + Send + Sync + 'static>
    MultipoolIndexer<BoxTransport, P, R>
{
    pub async fn new(
        contract_address: Address,
        factory_address: Address,
        provider: P,
        from_block: BlockNumberOrTag,
        raw_storage: R,
        multipool_storage: crate::multipool_storage::MultipoolStorage,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            contract_instance: contracts::Multipool::new(contract_address, provider.clone()),
            factory_contract_address: factory_address,
            chain_id: provider.get_chain_id().await?.to_string(),
            from_block,
            raw_storage,
            provider,
            multipool_storage,
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        self.spawn_multipool_creation_event_filter().await?;
        self.spawn_main_event_filters().await?;
        self.spawn_block_watcher().await?;

        Ok(())
    }

    pub async fn spawn_multipool_creation_event_filter(&self) -> anyhow::Result<()> {
        let factory_address = self.factory_contract_address.clone();
        let provider = self.provider.clone();
        let from_block = self.from_block.clone();
        let chain_id = self.chain_id.clone();
        let raw_storage = self.raw_storage.clone();
        let multipool_storage = self.multipool_storage.clone();

        tokio::spawn(async move {
            let factory_instance = MultipoolFactoryInstance::new(factory_address, provider);
            let mut multipool_creation_filter = factory_instance
                .MultipoolSpawned_filter()
                .from_block(from_block)
                .watch()
                .await
                .unwrap()
                .into_stream();
            loop {
                tokio::select! {
                    Some(Ok((event, log))) = multipool_creation_filter.next() => {
                        raw_storage.insert_event(
                            &factory_instance.address().to_string(),
                            &chain_id,
                            log.block_number.unwrap().try_into().unwrap(),
                            MultipoolSpawnedEvent {
                                address: event._0,
                                number: event.number,
                            },
                        )
                        .await
                        .unwrap();
                        multipool_storage.insert_multipool(event._0, U64::from(log.block_number.unwrap())).unwrap();
                    }
                    else => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn spawn_main_event_filters(&self) -> anyhow::Result<()> {
        let contract_instance = self.contract_instance.clone();
        let from_block = self.from_block.clone();
        let chain_id = self.chain_id.clone();
        let raw_storage = self.raw_storage.clone();

        let mut asset_change_filter = contract_instance
            .AssetChange_filter()
            .from_block(from_block)
            .watch()
            .await?
            .into_stream();
        let mut target_share_change_filter = contract_instance
            .TargetShareChange_filter()
            .from_block(from_block)
            .watch()
            .await?
            .into_stream();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(Ok((event, log))) = asset_change_filter.next() => {
                        raw_storage.insert_event(
                            &contract_instance.address().to_string(),
                            &chain_id,
                            log.block_number.unwrap().try_into().unwrap(),
                            AssetChangeEvent {
                                asset: event.asset,
                                quantity: event.quantity,
                                collected_cashbacks: U256::from(event.collectedCashbacks),
                            },
                        )
                        .await
                        .unwrap();
                    }
                    Some(Ok((event, log))) = target_share_change_filter.next() => {
                        raw_storage.insert_event(
                            &contract_instance.address().to_string(),
                            &chain_id,
                            log.block_number.unwrap().try_into().unwrap(),
                            TargetShareChangeEvent {
                                asset: event.asset,
                                new_target_share: event.newTargetShare,
                                new_total_target_shares: event.newTotalTargetShares,
                            },
                        )
                        .await
                        .unwrap();
                    }
                    else => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn spawn_block_watcher(&self) -> anyhow::Result<()> {
        let provider = self.provider.clone();
        let chain_id = self.chain_id.clone();
        let raw_storage = self.raw_storage.clone();

        let mut block_watcher = provider.subscribe_blocks().await?.into_stream();

        tokio::spawn(async move {
            while let Some(new_block_header) = block_watcher.next().await {
                raw_storage
                    .update_last_observed_block_number(
                        &chain_id,
                        new_block_header.number.try_into().unwrap(),
                    )
                    .await
                    .unwrap();
            }
        });

        Ok(())
    }
}
