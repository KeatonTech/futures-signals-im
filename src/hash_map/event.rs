use crate::structural_signal::pull_source::PullableDiff;
use crate::structural_signal::structural_signal_ext::SnapshottableEvent;
use core::hash::Hash;
use im::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapDiff<K> {
    Replace {},

    Insert { key: K },

    Update { key: K },

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
            MapDiff::Insert { key } | MapDiff::Remove { key } | MapDiff::Update { key } => {
                Some(key)
            }
            MapDiff::Replace {} | MapDiff::Clear {} => None,
        }
    }

    fn merge_with_previous(self, previous: MapDiff<K>) -> Option<MapDiff<K>> {
        if let MapDiff::Insert {key} = previous {
            // Insert then Remove => Nothing
            if let MapDiff::Remove {key: _} = self {
                return None;
            }

            // Insert then Update => Insert
            if let MapDiff::Update {key: _} = self {
                return Some(MapDiff::Insert {key});
            }
        }
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
