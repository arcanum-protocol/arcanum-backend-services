use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    chain_workers::{multipool_events::EventPoller, multipool_prices::PricePoller},
    config::BotConfig,
    multipool_storage::Multipool,
    rpc_controller::RpcRobber,
};

pub async fn run(config: BotConfig) -> HashMap<String, Arc<RwLock<Multipool>>> {
    let mut multipools = HashMap::new();
    for config in config.chains {
        let rpc = RpcRobber::new(config.rpc);
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
                        target_share_fetch_interval: 0,
                    }
                    .init(multipool.initial_assets, false, true)
                    .await
                    .unwrap()
                });
            }
            multipools.insert(id, storage);
        }
    }
    multipools
}
