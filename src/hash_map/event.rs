use crate::structural_signal_ext::SnapshottableEvent;
use core::hash::Hash;
use im::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapDiff<K> {
    Replace {},

    Insert { key: K },

    Remove { key: K },

    Clear {},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMapEvent<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    pub snapshot: HashMap<K, V>,
    pub diff: MapDiff<K>,
}

impl<K, V> SnapshottableEvent for HashMapEvent<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    type SnapshotType = HashMap<K, V>;

    fn snapshot(&self) -> Self::SnapshotType {
        self.snapshot.clone()
    }
}
