use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct MayBeExpired<V>(V, u64);

impl<V> MayBeExpired<V> {
    pub fn merge<V2, V3, F: FnOnce(V, V2) -> V3>(
        self,
        other: MayBeExpired<V2>,
        f: F,
    ) -> MayBeExpired<V3> {
        MayBeExpired(f(self.0, other.0), self.1.min(other.1))
    }

    pub fn not_older_than(self, interval: u64) -> Option<V> {
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Shold be always after epoch start")
            .as_secs();
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
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Shold be always after epoch start")
            .as_secs();
        self.1 = current_timestamp;
    }
}

impl<V> MayBeExpired<Option<V>> {
    pub fn transpose(self) -> Option<MayBeExpired<V>> {
        self.0.map(|val| MayBeExpired(val, self.1))
    }
}

impl<V> From<V> for MayBeExpired<V> {
    fn from(value: V) -> Self {
        Self(
            value,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Shold be always after epoch start")
                .as_secs(),
        )
    }
}
