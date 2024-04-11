use std::{
    ops::Add,
    time::{SystemTime, UNIX_EPOCH},
};

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

#[cfg(not(target_arch = "wasm32-unknown-unknown"))]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct StdTimeExtractor;

#[cfg(not(target_arch = "wasm32-unknown-unknown"))]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct WasmTimeExtractor;

impl TimeExtractor for WasmTimeExtractor {
    type TimeMeasure = u64;
    fn now() -> u64 {
        0
    }
}

impl TimeExtractor for StdTimeExtractor {
    type TimeMeasure = u64;
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Shold be always after epoch start")
            .as_secs()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MayBeExpired<V, T: TimeExtractor>(V, T::TimeMeasure);

impl<V: Clone, T: TimeExtractor> Clone for MayBeExpired<V, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }
}

impl<V, T: TimeExtractor> MayBeExpired<V, T> {
    pub fn new(value: V) -> Self {
        MayBeExpired(value, T::now())
    }

    pub fn with_time(value: V, timestamp: T::TimeMeasure) -> Self {
        Self(value, timestamp)
    }

    pub fn not_older_than(self, interval: T::TimeMeasure) -> Option<V> {
        let current_timestamp = T::now();
        if current_timestamp <= self.1 + interval {
            Some(self.0)
        } else {
            None
        }
    }

    pub fn any_age(self) -> V {
        self.0
    }

    pub fn refresh(&mut self) {
        let current_timestamp = T::now();
        self.1 = current_timestamp;
    }
}

impl<V, T: TimeExtractor> MayBeExpired<Option<V>, T> {
    pub fn transpose(self) -> Option<MayBeExpired<V, T>> {
        self.0.map(|val| MayBeExpired(val, self.1))
    }
}

impl<V, T: TimeExtractor, E> MayBeExpired<Result<V, E>, T> {
    pub fn transpose(self) -> Result<MayBeExpired<V, T>, E> {
        self.0.map(|value| MayBeExpired(value, self.1))
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
                let temp_vec: Vec<T::TimeMeasure> = vec![$($type.1.clone()),*];
                let min_timestamp = temp_vec.iter().min().expect("not enought elements");
                MayBeExpired(operator(($($type.0),*)), *min_timestamp)
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
            MayBeExpired("New data", time),
        );
        let t2_merged = t2.merge(|(a, b)| a.to_owned() + b);
        assert_eq!(
            t2_merged,
            MayBeExpiredStd::new("New dataNew data".to_string())
        );
        let t3 = (
            MayBeExpired::<_, StdTimeExtractor>(1, 1),
            MayBeExpired(2, 2),
            MayBeExpired(3, 3),
        );
        let t3_merged = t3.merge(|(a, b, c)| a + b + c);
        let t4 = (
            MayBeExpired::<_, StdTimeExtractor>(1, 1),
            MayBeExpired(2, 2),
            MayBeExpired(3, 3),
            MayBeExpired(4, 4),
        );
        let t4_merged = t4.merge(|(a, b, c, d)| a + b + c + d);
        let t5 = (
            MayBeExpired::<_, StdTimeExtractor>(1, time - 20),
            MayBeExpired(2, time - 11),
            MayBeExpired(3, time + 1113),
            MayBeExpired(4, time - 1),
            MayBeExpired(5, time),
        );
        let t5_merged = t5.merge(|(a, b, c, d, _)| a + b + c + d);
        let t6 = (
            MayBeExpired::<_, StdTimeExtractor>(1, 1),
            MayBeExpired(2, 2),
            MayBeExpired(3, 3),
            MayBeExpired(4, 4),
            MayBeExpired(5, 1),
            MayBeExpired(6, 2),
            MayBeExpired(7, 3),
            MayBeExpired(8, 4),
            MayBeExpired(9, 1),
            MayBeExpired(10, 2),
            MayBeExpired(11, 3),
            MayBeExpired(12, 4),
        );
        let t6_merged = t6.merge(|(a, b, c, d, e, f, g, k, l, m, n, o)| {
            a + b + c + d + e + f + g + k + l + m + n + o
        });
        assert_eq!(t3_merged, MayBeExpired(6, 1));
        assert_eq!(t4_merged, MayBeExpired(10, 1));
        assert_eq!(t5_merged, MayBeExpired(10, time - 20));
        assert_eq!(t6_merged, MayBeExpired(78, 1));
    }
}
