pub mod multipool {
    ethers::prelude::abigen!(MultipoolContract, "./src/abi/multipool.json");
}

pub mod multipool_factory {
    ethers::prelude::abigen!(MultipoolFactoryContract, "./src/abi/multipool_factory.json");
}
