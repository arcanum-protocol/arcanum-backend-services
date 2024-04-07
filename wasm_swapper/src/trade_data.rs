use ethers::prelude::*;
use std::collections::HashMap;

pub struct UniswapPool {
    fee: u32,
    address: Address,
}

pub struct UniswapAssetPools {
    asset_symbol: String,
    base_is_token0: bool,
    pools: Vec<UniswapPool>,
}

pub struct SiloData {
    pool_address: Address,
    base_address: Address,
}

fn parse_address(address: &str) -> Address {
    address.parse().expect("Failed to parse address")
}

pub static SILO_POOLS: HashMap<Address, SiloData> = [
    (
        parse_address("0x96E1301bd2536A3C56EBff8335FD892dD9bD02dC"),
        SiloData {
            pool_address: parse_address("0xDe998E5EeF06dD09fF467086610B175F179A66A0"),
            base_address: parse_address("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8"),
        },
    ),
    (
        parse_address("0xE9B35c753b6Ec9b5a4bBd8c385d16cDb19517185"),
        SiloData {
            pool_address: parse_address("0x19d3F8D09773065867e9fD11716229e73481c55A"),
            base_address: parse_address("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8"),
        },
    ),
    (
        parse_address("0xAf06C6106D3a202AD53a4584189e3Dd37E4D2735"),
        SiloData {
            pool_address: parse_address("0xaee935408b94bae1Ce4eA15d22b3cA33c91eFe81"),
            base_address: parse_address("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8"),
        },
    ),
    (
        parse_address("0x51DdFa50752782089d032DD293e4650dAf16F151"),
        SiloData {
            pool_address: parse_address("0x5C2B80214c1961dB06f69DD4128BcfFc6423d44F"),
            base_address: parse_address("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8"),
        },
    ),
]
.into();

pub static UNISWAP_POOLS: HashMap<Address, UniswapAssetPools> = [
    (
        parse_address("0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f"),
        UniswapAssetPools {
            base_is_token0: true,
            asset_symbol: "WBTC".into(),
            pools: vec![
                UniswapPool {
                    fee: 500,
                    address: parse_address("0x2f5e87c9312fa29aed5c179e456625d79015299c"),
                },
                UniswapPool {
                    fee: 3000,
                    address: parse_address("0x149e36e72726e0bcea5c59d40df2c43f60f5a22d"),
                },
                UniswapPool {
                    fee: 100,
                    address: parse_address("0x03a3be7ab4aa263d42d63b6cc594f4fb3d3f3951"),
                },
            ],
        },
    ),
    (
        parse_address("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8"),
        UniswapAssetPools {
            base_is_token0: false,
            asset_symbol: "USDCE".into(),
            pools: vec![UniswapPool {
                fee: 500,
                address: parse_address("0xC31E54c7a869B9FcBEcc14363CF510d1c41fa443"),
            }],
        },
    ),
    (
        parse_address("0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9"),
        UniswapAssetPools {
            base_is_token0: false,
            asset_symbol: "USDT".into(),
            pools: vec![
                UniswapPool {
                    fee: 500,
                    address: parse_address("0x641c00a822e8b671738d32a431a4fb6074e5c79d"),
                },
                UniswapPool {
                    fee: 3000,
                    address: parse_address("0xc82819f72a9e77e2c0c3a69b3196478f44303cf4"),
                },
                UniswapPool {
                    fee: 100,
                    address: parse_address("0x58039203442c9f2a45d5536bd021a383c7f3035c"),
                },
            ],
        },
    ),
]
.into();

//eth_pools:
//  - address: "0x5979d7b546e38e414f7e9822514be443a4800529"
//    asset_symbol: "wstETH"
//    base_is_token0: true
//    pools:
//      - fee: 100
//        address: "0x35218a1cbac5bbc3e57fd9bd38219d37571b3537"
//      - fee: 3000
//        address: "0x7103b8f34473c7812818c55eb127d1f590f67d84"
//      - fee: 10000
//        address: "0x99ac8ca7087fa4a2a1fb6357269965a2014abc35"
//
//  - address: "0xfc5a1a6eb076a2c7ad06ed22c90d7e710e35ad0a"
//    asset_symbol: "GMX"
//    base_is_token0: false
//    pools:
//      - fee: 10000
//        address: "0x80a9ae39310abf666a87c743d6ebbd0e8c42158e"
//      - fee: 3000
//        address: "0x1aeedd3727a6431b8f070c0afaa81cc74f273882"
//      - fee: 500
//        address: "0xb435ebfe0bf4ce66810aa4d44e3a5ca875d40db1"
//  - address: "0x3082cc23568ea640225c2467653db90e9250aaa0"
//    asset_symbol: "RDNT"
//    base_is_token0: true
//    pools:
//      - fee: 10000
//        address: "0xa8ba5f3ccfb8d2b7f4225e371cde11871e088933"
//      - fee: 3000
//        address: "0x446BF9748B4eA044dd759d9B9311C70491dF8F29"
//  - address: "0x0341c0c0ec423328621788d4854119b97f44e391"
//    asset_symbol: "SILO"
//    base_is_token0: true
//    pools:
//      - fee: 10000
//        address: "0xd3e11119d2680c963f1cdcffece0c4ade823fb58"
//  - address: "0x539bde0d7dbd336b79148aa742883198bbf60342"
//    asset_symbol: "MAGIC"
//    base_is_token0: true
//    pools:
//      - fee: 10000
//        address: "0x7e7fb3cceca5f2ac952edf221fd2a9f62e411980"
//      - fee: 3000
//        address: "0x59d72ddb29da32847a4665d08ffc8464a7185fae"
//      - fee: 500
//        address: "0xf44f17a6fc5d2f0ad1ff47e682570fa5a8eb9050"
//  - address: "0x51fc0f6660482ea73330e414efd7808811a57fa2"
//    asset_symbol: "PREMIA"
//    base_is_token0: true
//    pools:
//      - fee: 10000
//        address: "0x4b220d875a5951244896b2f3d7f1545b2a3d0d5f"
//  - address: "0x0c880f6761f1af8d9aa9c466984b80dab9a8c9e8"
//    asset_symbol: "PENDLE"
//    base_is_token0: true
//    pools:
//      - fee: 10000
//        address: "0xe8629b6a488f366d27dad801d1b5b445199e2ada"
//      - fee: 3000
//        address: "0xdbaeb7f0dfe3a0aafd798ccecb5b22e708f7852c"
