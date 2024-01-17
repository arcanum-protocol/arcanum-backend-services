//use ethers::prelude::*;
//use futures::{Future, FutureExt};
//use std::pin::Pin;
//use std::sync::Arc;
//use tokio::sync::Semaphore;
//
//trait Rpc {
//    fn call<T, F: Future<Output = T>, A: FnOnce(Arc<Provider<Http>>) -> F>(
//        &self,
//        action: A,
//    ) -> Pin<Box<dyn Future<Output = T> + Send>>;
//}
//
//pub struct EmbeddedRpc {
//    parallel_use: Semaphore,
//    provider: Arc<Provider<Http>>,
//}
//
//impl Rpc for EmbeddedRpc {
//    fn call<T, F: Future<Output = T>, A: FnOnce(Arc<Provider<Http>>) -> F>(
//        &self,
//        action: F,
//    ) -> Pin<Box<dyn Future<Output = T> + Send>> {
//        async {
//            self.parallel_use.acquire();
//            action()
//        }
//        .boxed()
//    }
//}
