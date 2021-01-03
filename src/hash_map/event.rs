use crate::structural_signal::structural_signal_ext::SnapshottableEvent;
use crate::structural_signal::pull_source::PullableDiff;
use core::hash::Hash;
use im::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapDiff<K> {
    Replace {},

    Insert { key: K },

    Remove { key: K },

    Clear {},
}

impl<K> PullableDiff for MapDiff<K>
where
    K: Clone + Eq + Hash,
{
    type KeyType = K;

    fn get_key(&self) -> Option<&K> {
        match self {
            MapDiff::Insert { key } | MapDiff::Remove { key } => Some(key),
            MapDiff::Replace {} | MapDiff::Clear {} => None,
        }
    }

    fn merge_with_previous(self, _previous: MapDiff<K>) -> Option<MapDiff<K>> {
        Some(self)
    }

    fn full_replace() -> MapDiff<K> {
        MapDiff::Replace {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMapEvent<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    pub snapshot: HashMap<K, V>,
    pub diffs: Vec<MapDiff<K>>,
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
