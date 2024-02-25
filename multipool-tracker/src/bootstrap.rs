use std::{collections::BTreeMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    chain_workers::{multipool_events::EventPoller, multipool_prices::PricePoller},
    config::BotConfig,
    multipool::Multipool,
    rpc_controller::RpcRobber,
};

pub async fn run(config: BotConfig) -> (MultipoolStorage, RpcRobber) {
    let mut multipools = BTreeMap::new();
    let mut rpcs = Vec::new();
    let rpc = RpcRobber::new(config.rpc);
    rpcs.push(rpc.clone());
    for (id, multipool) in config.multipools {
        let storage = Arc::new(RwLock::new(Multipool::new(multipool.contract_address)));
        {
            let rpc = rpc.clone();
            let storage = storage.clone();
            tokio::spawn(async move {
                PricePoller {
                    rpc,
                    multipool_storage: storage,
                    fetch_interval: multipool.price_fetcher.interval,
                }
                .init()
                .await
                .unwrap()
            });
        }
        {
            let rpc = rpc.clone();
            let storage = storage.clone();
            tokio::spawn(async move {
                EventPoller {
                    rpc,
                    multipool_storage: storage,
                    quantity_fetch_interval: multipool.event_fetcher.interval,
                    target_share_fetch_interval: multipool.event_fetcher.interval,
                }
                .init(multipool.initial_assets, true, true)
                .await
                .unwrap()
            });
        }
        multipools.insert(id, storage);
    }
    (MultipoolStorage { inner: multipools }, rpc)
}
