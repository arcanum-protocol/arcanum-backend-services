use std::{
    ops::Add,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::borsh_methods::{deserialize, serialize};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub trait TimeExtractor {
    type TimeMeasure: Ord
        + Add<Output = Self::TimeMeasure>
        + Clone
        + Copy
        + DeserializeOwned
        + Serialize
        + PartialEq
        + std::fmt::Debug;
    fn now() -> Self::TimeMeasure;
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct EmptyTimeExtractor;

impl TimeExtractor for EmptyTimeExtractor {
    type TimeMeasure = u64;
    fn now() -> u64 {
        panic!("Should never be invoked");
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct StdTimeExtractor;

impl TimeExtractor for StdTimeExtractor {
    type TimeMeasure = u64;
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Shold be always after epoch start")
            .as_secs()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MayBeExpired<V, T: TimeExtractor> {
    pub value: V,
    pub timestamp: T::TimeMeasure,
}

impl BorshDeserialize for MayBeExpired<alloy::primitives::U256, EmptyTimeExtractor> {
    // Required method
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let value = deserialize::u256(reader)?;
        let timestamp =
            borsh::BorshDeserialize::deserialize_reader(reader).map(u64::from_le_bytes)?;
        Ok(Self { value, timestamp })
    }
}

impl BorshSerialize for MayBeExpired<alloy::primitives::U256, EmptyTimeExtractor> {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        serialize::u256(&self.value, writer)?;
        borsh::BorshSerialize::serialize(&self.timestamp.to_le_bytes(), writer)
    }
}

impl<V: Clone, T: TimeExtractor> Clone for MayBeExpired<V, T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            timestamp: self.timestamp,
        }
    }
}

impl<V, T: TimeExtractor> MayBeExpired<V, T> {
    pub fn new(value: V) -> Self {
        MayBeExpired {
            value,
            timestamp: T::now(),
        }
    }

    pub fn build<T1: TimeExtractor<TimeMeasure = T::TimeMeasure>>(value: V) -> Self {
        MayBeExpired {
            value,
            timestamp: T1::now(),
        }
    }

    pub fn with_time(value: V, timestamp: T::TimeMeasure) -> Self {
        Self { value, timestamp }
    }

    pub fn not_older_than<E: TimeExtractor<TimeMeasure = T::TimeMeasure>>(
        self,
        interval: T::TimeMeasure,
    ) -> Option<V> {
        let current_timestamp = E::now();
        if current_timestamp <= self.timestamp + interval {
            Some(self.value)
        } else {
            None
        }
    }

    pub fn any_age(self) -> V {
        self.value
    }

    pub fn map<U, F: FnOnce(V) -> U>(self, f: F) -> MayBeExpired<U, T> {
        MayBeExpired {
            value: f(self.value),
            timestamp: self.timestamp,
        }
    }

    pub fn refresh(&mut self) {
        let current_timestamp = T::now();
        self.timestamp = current_timestamp;
    }
}

impl<V, T: TimeExtractor> MayBeExpired<Option<V>, T> {
    pub fn transpose(self) -> Option<MayBeExpired<V, T>> {
        self.value.map(|value| MayBeExpired {
            value,
            timestamp: self.timestamp,
        })
    }
}

impl<V, T: TimeExtractor, E> MayBeExpired<Result<V, E>, T> {
    pub fn transpose(self) -> Result<MayBeExpired<V, T>, E> {
        self.value.map(|value| MayBeExpired {
            value,
            timestamp: self.timestamp,
        })
    }
}

pub trait Merge: Sized {
    type Item;
    type Measure: TimeExtractor;
    fn merge<V, F: FnOnce(Self::Item) -> V>(self, operator: F) -> MayBeExpired<V, Self::Measure>;
}

macro_rules! impl_merge {
    ($($type:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($type),*, T: TimeExtractor> Merge for ($(MayBeExpired<$type, T>),*)
        {
            type Item = ($($type),*);
            type Measure = T;
            fn merge<V, F:FnOnce(Self::Item) -> V>(self, operator: F) -> MayBeExpired<V, T> {
                let ($($type),*) = self;
                let temp_vec: Vec<T::TimeMeasure> = vec![$($type.timestamp.clone()),*];
                let min_timestamp = temp_vec.iter().min().expect("not enought elements");
                MayBeExpired {value: operator(($($type.value),*)), timestamp: *min_timestamp }
            }
        }
    };
}

impl_merge!(T1, T2);
impl_merge!(T1, T2, T3);
impl_merge!(T1, T2, T3, T4);
impl_merge!(T1, T2, T3, T4, T5);
impl_merge!(T1, T2, T3, T4, T5, T6);
impl_merge!(T1, T2, T3, T4, T5, T6, T7);
impl_merge!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_merge!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_merge!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_merge!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_merge!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

#[cfg(test)]
mod tests {
    use super::*;
    type MayBeExpiredStd<V> = MayBeExpired<V, StdTimeExtractor>;

    #[test]
    fn merge_test() {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Shold be always after epoch start")
            .as_secs();
        let t2 = (
            MayBeExpiredStd::new("New data"),
            MayBeExpired {
                value: "New data",
                timestamp: time,
            },
        );
        let t2_merged = t2.merge(|(a, b)| a.to_owned() + b);
        assert_eq!(
            t2_merged,
            MayBeExpiredStd::new("New dataNew data".to_string())
        );
        let t3 = (
            MayBeExpired::<_, StdTimeExtractor> {
                value: 1,
                timestamp: 1,
            },
            MayBeExpired {
                value: 2,
                timestamp: 2,
            },
            MayBeExpired {
                value: 3,
                timestamp: 3,
            },
        );
        let t3_merged = t3.merge(|(a, b, c)| a + b + c);
        let t4 = (
            MayBeExpired::<_, StdTimeExtractor> {
                value: 1,
                timestamp: 1,
            },
            MayBeExpired {
                value: 2,
                timestamp: 2,
            },
            MayBeExpired {
                value: 3,
                timestamp: 3,
            },
            MayBeExpired {
                value: 4,
                timestamp: 4,
            },
        );
        let t4_merged = t4.merge(|(a, b, c, d)| a + b + c + d);
        let t5 = (
            MayBeExpired::<_, StdTimeExtractor> {
                value: 1,
                timestamp: time - 20,
            },
            MayBeExpired {
                value: 2,
                timestamp: time - 11,
            },
            MayBeExpired {
                value: 3,
                timestamp: time + 1113,
            },
            MayBeExpired {
                value: 4,
                timestamp: time - 1,
            },
            MayBeExpired {
                value: 5,
                timestamp: time,
            },
        );
        let t5_merged = t5.merge(|(a, b, c, d, _)| a + b + c + d);
        let t6 = (
            MayBeExpired::<_, StdTimeExtractor> {
                value: 1,
                timestamp: 1,
            },
            MayBeExpired {
                value: 2,
                timestamp: 2,
            },
            MayBeExpired {
                value: 3,
                timestamp: 3,
            },
            MayBeExpired {
                value: 4,
                timestamp: 4,
            },
            MayBeExpired {
                value: 5,
                timestamp: 1,
            },
            MayBeExpired {
                value: 6,
                timestamp: 2,
            },
            MayBeExpired {
                value: 7,
                timestamp: 3,
            },
            MayBeExpired {
                value: 8,
                timestamp: 4,
            },
            MayBeExpired {
                value: 9,
                timestamp: 1,
            },
            MayBeExpired {
                value: 10,
                timestamp: 2,
            },
            MayBeExpired {
                value: 11,
                timestamp: 3,
            },
            MayBeExpired {
                value: 12,
                timestamp: 4,
            },
        );
        let t6_merged = t6.merge(|(a, b, c, d, e, f, g, k, l, m, n, o)| {
            a + b + c + d + e + f + g + k + l + m + n + o
        });
        assert_eq!(
            t3_merged,
            MayBeExpired {
                value: 6,
                timestamp: 1
            }
        );
        assert_eq!(
            t4_merged,
            MayBeExpired {
                value: 10,
                timestamp: 1
            }
        );
        assert_eq!(
            t5_merged,
            MayBeExpired {
                value: 10,
                timestamp: time - 20
            }
        );
        assert_eq!(
            t6_merged,
            MayBeExpired {
                value: 78,
                timestamp: 1
            }
        );
    }
}
