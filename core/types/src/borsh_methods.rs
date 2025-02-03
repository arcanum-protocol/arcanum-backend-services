use alloy::primitives::{
    aliases::{U112, U128, U96},
    Address, B256, U256,
};

pub mod deserialize {
    use super::*;

    pub fn u256<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<U256, borsh::io::Error> {
        borsh::BorshDeserialize::deserialize_reader(reader).map(U256::from_le_bytes::<32>)
    }

    pub fn address<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<Address, borsh::io::Error> {
        borsh::BorshDeserialize::deserialize_reader(reader).map(|a: [u8; 20]| Address::from(a))
    }

    pub fn b256<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<B256, borsh::io::Error> {
        borsh::BorshDeserialize::deserialize_reader(reader).map(|a: [u8; 32]| B256::from(a))
    }

    pub fn u96<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<U96, borsh::io::Error> {
        borsh::BorshDeserialize::deserialize_reader(reader).map(U96::from_le_bytes::<12>)
    }

    pub fn u112<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<U112, borsh::io::Error> {
        borsh::BorshDeserialize::deserialize_reader(reader).map(U112::from_le_bytes::<14>)
    }

    pub fn u128<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<U128, borsh::io::Error> {
        borsh::BorshDeserialize::deserialize_reader(reader).map(U128::from_le_bytes::<16>)
    }
}
pub mod serialize {
    use super::*;

    pub fn u256<W: borsh::io::Write>(
        obj: &U256,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        borsh::BorshSerialize::serialize(&obj.to_le_bytes::<32>(), writer)
    }

    pub fn address<W: borsh::io::Write>(
        obj: &Address,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        borsh::BorshSerialize::serialize(&<[u8; 20]>::from(obj.0), writer)
    }

    pub fn b256<W: borsh::io::Write>(
        obj: &B256,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        borsh::BorshSerialize::serialize(&obj.0, writer)
    }

    pub fn u96<W: borsh::io::Write>(
        obj: &U96,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        borsh::BorshSerialize::serialize(&obj.to_le_bytes::<12>(), writer)
    }

    pub fn u112<W: borsh::io::Write>(
        obj: &U112,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        borsh::BorshSerialize::serialize(&obj.to_le_bytes::<14>(), writer)
    }

    pub fn u128<W: borsh::io::Write>(
        obj: &U128,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        borsh::BorshSerialize::serialize(&obj.to_le_bytes::<16>(), writer)
    }
}
