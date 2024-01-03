use ethers::{abi::Detokenize, prelude::ContractCall, providers::Middleware};
use tokio::sync::mpsc;

pub type ChainFetchTask<M: Middleware> = ContractCall<M, Box<dyn Detokenize>>;

pub struct TaskPlanner<M: Middleware> {
    planner: mpsc::Sender<ChainFetchTask<M>>,
}

impl<M: Middleware> TaskPlanner<M> {}

pub struct ChainAdapter<M: Middleware> {
    tasks: mpsc::Receiver<ChainFetchTask<M>>,
}

impl<M: Middleware> ChainAdapter<M> {
    pub fn run_adapter(queue_capacity: usize) -> (Self, TaskPlanner<M>) {
        let (planner, tasks) = mpsc::channel(queue_capacity);
        (Self { tasks }, TaskPlanner { planner })
    }
}
