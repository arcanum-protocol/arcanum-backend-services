use ethers::prelude::*;
use std::ops::{Add, Div, Shl};

pub fn get_price_from_tick(tick: I256) -> U256 {
    let abs_tick: U256 = U256::from_dec_str(&tick.abs().to_string()).unwrap();
    assert!(
        abs_tick.lt(&U256::from_dec_str(&I256::MAX.to_string()).unwrap()),
        "FUCK"
    );
    let mut ratio = if !(abs_tick & U256::from(0x1)).is_zero() {
        U256::from(0xfffcb933bd6fad37aa2d162d1a594001u128)
    } else {
        U256::from_dec_str("340282366920938463463374607431768211456")
            .expect("Should be valid number")
    };
    if !(abs_tick & U256::from(0x2)).is_zero() {
        ratio = (ratio * U256::from(0xfff97272373d413259a46990580e213au128)) >> 128
    }
    if !(abs_tick & U256::from(0x4)).is_zero() {
        ratio = (ratio * U256::from(0xfff2e50f5f656932ef12357cf3c7fdccu128)) >> 128
    }
    if !(abs_tick & U256::from(0x8)).is_zero() {
        ratio = (ratio * U256::from(0xffe5caca7e10e4e61c3624eaa0941cd0u128)) >> 128
    }
    if !(abs_tick & U256::from(0x10)).is_zero() {
        ratio = (ratio * U256::from(0xfff2e50f5f656932ef12357cf3c7fdccu128)) >> 128
    }
    if !(abs_tick & U256::from(0x20)).is_zero() {
        ratio = (ratio * U256::from(0xff973b41fa98c081472e6896dfb254c0u128)) >> 128
    }
    if !(abs_tick & U256::from(0x40)).is_zero() {
        ratio = (ratio * U256::from(0xff2ea16466c96a3843ec78b326b52861u128)) >> 128
    }
    if !(abs_tick & U256::from(0x80)).is_zero() {
        ratio = (ratio * U256::from(0xfe5dee046a99a2a811c461f1969c3053u128)) >> 128
    }
    if !(abs_tick & U256::from(0x100)).is_zero() {
        ratio = (ratio * U256::from(0xfcbe86c7900a88aedcffc83b479aa3a4u128)) >> 128
    }
    if !(abs_tick & U256::from(0x200)).is_zero() {
        ratio = (ratio * U256::from(0xf987a7253ac413176f2b074cf7815e54u128)) >> 128
    }
    if !(abs_tick & U256::from(0x400)).is_zero() {
        ratio = (ratio * U256::from(0xf3392b0822b70005940c7a398e4b70f3u128)) >> 128
    }
    if !(abs_tick & U256::from(0x800)).is_zero() {
        ratio = (ratio * U256::from(0xe7159475a2c29b7443b29c7fa6e889d9u128)) >> 128
    }
    if !(abs_tick & U256::from(0x1000)).is_zero() {
        ratio = (ratio * U256::from(0xd097f3bdfd2022b8845ad8f792aa5825u128)) >> 128
    }
    if !(abs_tick & U256::from(0x2000)).is_zero() {
        ratio = (ratio * U256::from(0xa9f746462d870fdf8a65dc1f90e061e5u128)) >> 128
    }
    if !(abs_tick & U256::from(0x4000)).is_zero() {
        ratio = (ratio * U256::from(0x70d869a156d2a1b890bb3df62baf32f7u128)) >> 128
    }
    if !(abs_tick & U256::from(0x8000)).is_zero() {
        ratio = (ratio * U256::from(0x31be135f97d08fd981231505542fcfa6u128)) >> 128
    }
    if !(abs_tick & U256::from(0x10000)).is_zero() {
        ratio = (ratio * U256::from(0x9aa508b5b7a84e1c677de54f3e99bc9u128)) >> 128
    }
    if !(abs_tick & U256::from(0x20000)).is_zero() {
        ratio = (ratio * U256::from(0x5d6af8dedb81196699c329225ee604u128)) >> 128
    }
    if !(abs_tick & U256::from(0x40000)).is_zero() {
        ratio = (ratio * U256::from(0x2216e584f5fa1ea926041bedfe98u128)) >> 128
    }
    if !(abs_tick & U256::from(0x80000)).is_zero() {
        ratio = (ratio * U256::from(0x48a170391f7dc42444e8fa2u128)) >> 128
    }
    if tick.gt(&I256::zero()) {
        ratio = U256::MAX.div(ratio);
    }
    let dig = if (ratio % U256::from(1).shl(32)).is_zero() {
        U256::from(0)
    } else {
        U256::from(1)
    };
    (ratio >> 32).add(dig)
}
