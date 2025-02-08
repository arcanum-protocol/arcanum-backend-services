use anyhow::Result;
use futures::Future;
use multipool::Multipool;
use tokio::task::JoinHandle;

pub trait HookInitializer {
    fn initialize_hook<F: Fn() -> Multipool + Send + Sync + 'static>(
        &mut self,
        multipool: F,
    ) -> impl Future<Output = JoinHandle<Result<()>>>;
}
