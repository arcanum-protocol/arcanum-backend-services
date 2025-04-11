use alloy::primitives::{
    address,
    aliases::{I24, U24, U256},
    b256, keccak256, Address, B256,
};
use alloy_sol_types::SolValue;

pub const FACTORY_ADDRESS: Address = address!("7eFe6656d08f2d6689Ed8ca8b5A3DEA0efaa769f");
// pub const FACTORY_ADDRESS: Address = address!("248AB79Bbb9bC29bB72f7Cd42F17e054Fc40188e");
pub const POOL_INIT_CODE_HASH: B256 =
    b256!("e34f199b19b2b4f47f68442619d555527d244f78a3297ea89325f843f87b8b54");

/// The default factory enabled fee amounts, denominated in hundredths of bips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum FeeAmount {
    LOWEST = 100,
    LOW = 500,
    MEDIUM = 3000,
    HIGH = 10000,
}

impl FeeAmount {
    /// The default factory tick spacings by fee amount.
    #[inline]
    #[must_use]
    pub fn tick_spacing(&self) -> I24 {
        match self {
            Self::LOWEST => I24::ONE,
            Self::LOW => I24::from_limbs([10]),
            Self::MEDIUM => I24::from_limbs([60]),
            Self::HIGH => I24::from_limbs([200]),
        }
    }

    pub fn to_val(self) -> U256 {
        U256::from(match self {
            FeeAmount::LOWEST => 100,
            FeeAmount::LOW => 500,
            FeeAmount::MEDIUM => 3000,
            FeeAmount::HIGH => 10000,
        })
    }

    pub fn iter() -> impl Iterator<Item = FeeAmount> {
        [
            FeeAmount::LOWEST,
            FeeAmount::LOW,
            FeeAmount::MEDIUM,
            FeeAmount::HIGH,
        ]
        .iter()
        .copied()
    }
}

impl From<u32> for FeeAmount {
    #[inline]
    fn from(fee: u32) -> Self {
        match fee {
            100 => Self::LOWEST,
            500 => Self::LOW,
            3000 => Self::MEDIUM,
            10000 => Self::HIGH,
            _ => unreachable!("Wrong fee for v3"),
        }
    }
}

impl From<FeeAmount> for u32 {
    #[inline]
    fn from(fee: FeeAmount) -> Self {
        match fee {
            FeeAmount::LOWEST => 100,
            FeeAmount::LOW => 500,
            FeeAmount::MEDIUM => 3000,
            FeeAmount::HIGH => 10000,
        }
    }
}

impl From<i32> for FeeAmount {
    #[inline]
    fn from(tick_spacing: i32) -> Self {
        match tick_spacing {
            1 => Self::LOWEST,
            10 => Self::LOW,
            60 => Self::MEDIUM,
            200 => Self::HIGH,
            _ => unreachable!("Wrong spacing for v3"),
        }
    }
}

impl From<FeeAmount> for U24 {
    #[inline]
    fn from(fee: FeeAmount) -> Self {
        Self::from_limbs([match fee {
            FeeAmount::LOWEST => 100,
            FeeAmount::LOW => 500,
            FeeAmount::MEDIUM => 3000,
            FeeAmount::HIGH => 10000,
        }])
    }
}

impl From<FeeAmount> for U256 {
    #[inline]
    fn from(fee: FeeAmount) -> Self {
        U256::from(match fee {
            FeeAmount::LOWEST => 100,
            FeeAmount::LOW => 500,
            FeeAmount::MEDIUM => 3000,
            FeeAmount::HIGH => 10000,
        })
    }
}

impl From<U24> for FeeAmount {
    #[inline]
    fn from(fee: U24) -> Self {
        (fee.into_limbs()[0] as u32).into()
    }
}

pub fn compute_pool_address(
    factory: Address,
    token_a: Address,
    token_b: Address,
    fee: FeeAmount,
    init_code_hash_manual_override: Option<B256>,
) -> Address {
    assert_ne!(token_a, token_b, "ADDRESSES");
    let (token_0, token_1) = if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    };
    let fee: U24 = fee.into();
    let salt = keccak256((token_0, token_1, fee).abi_encode());

    factory.create2(
        salt,
        init_code_hash_manual_override.unwrap_or(POOL_INIT_CODE_HASH),
    )
}
