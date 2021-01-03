use super::event::{HashMapEvent, MapDiff};
use crate::structural_signal::pull_source::{
    PullSourceHost, PullSourceStructuralSignal, StructrualSignalPullSource,
};
use im::HashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::hash::Hash;
use std::iter::Iterator;
use std::sync::Arc;

/// The internal state of a MutableHashMap or MutableHashMapReader. All
/// clones (and readonly clones) will share this same instance.
///
/// The only difference between `MutableHashMap` and `MutableHashMapReader`
/// is that the latter provides no way to get a mutable reference to this
/// struct and therefore cannot perform any modification operation.
#[derive(Debug)]
pub struct MutableHashMapState<K: Clone + Eq + Hash, V: Clone> {
    hash_map: HashMap<K, V>,
    pull_source: StructrualSignalPullSource<MapDiff<K>>,
}

impl<K: Clone + Eq + Hash, V: Clone> PullSourceHost for MutableHashMapState<K, V> {
    type DiffType = MapDiff<K>;
    type EventType = HashMapEvent<K, V>;

    fn get_pull_source<'a>(&'a mut self) -> &'a mut StructrualSignalPullSource<Self::DiffType> {
        &mut self.pull_source
    }

    fn make_event(&self, diffs: Vec<Self::DiffType>) -> Self::EventType {
        HashMapEvent {
            snapshot: self.hash_map.clone(),
            diffs: diffs,
        }
    }
}

/// A HashMap that can be observed as it changes over time.
///
/// This structure is backed by `im.HashMap` and so requires that keys and values
/// are clonable. The backing structure is optimized to clone only when necessary.
pub struct MutableHashMap<K: Clone + Eq + Hash, V: Clone>(Arc<RwLock<MutableHashMapState<K, V>>>);

impl<K: Clone + Eq + Hash, V: Clone> Clone for MutableHashMap<K, V> {
    fn clone(&self) -> Self {
        MutableHashMap {
            0: Arc::new(RwLock::new(MutableHashMapState {
                hash_map: self.0.read().hash_map.clone(),
                pull_source: StructrualSignalPullSource::new(),
            })),
        }
    }
}

impl<K: Clone + Eq + Hash, V: Clone> MutableHashMap<K, V> {
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<MutableHashMapState<K, V>> {
        self.0.read()
    }

    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<MutableHashMapState<K, V>> {
        self.0.write()
    }

    pub fn new() -> Self {
        MutableHashMap {
            0: Arc::new(RwLock::new(MutableHashMapState {
                hash_map: HashMap::new(),
                pull_source: StructrualSignalPullSource::new(),
            })),
        }
    }

    /// Creates a read-only view into this data structure. This Reader object
    /// can lookup items in this map at any time, but cannot modify it.
    /// Readers can be cloned. Note that Readers can hold a ReadLock which can
    /// block MutableHashMap::write() calls, so use them with care.
    #[inline]
    pub fn reader(&self) -> MutableHashMapReader<K, V> {
        MutableHashMapReader { 0: self.0.clone() }
    }

    #[inline]
    pub fn as_signal(&self) -> PullSourceStructuralSignal<MutableHashMapState<K, V>> {
        PullSourceStructuralSignal::new(self.0.clone())
    }
}

/// A read-only view into a MutableHashMap.
pub struct MutableHashMapReader<K: Clone + Eq + Hash, V: Clone>(
    Arc<RwLock<MutableHashMapState<K, V>>>,
);

impl<K: Clone + Eq + Hash, V: Clone> Clone for MutableHashMapReader<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        MutableHashMapReader { 0: self.0.clone() }
    }
}

impl<K: Clone + Eq + Hash, V: Clone> MutableHashMapReader<K, V> {
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<MutableHashMapState<K, V>> {
        self.0.read()
    }

    #[inline]
    pub fn as_signal(&self) -> PullSourceStructuralSignal<MutableHashMapState<K, V>> {
        PullSourceStructuralSignal::new(self.0.clone())
    }
}

impl<K: Clone + Eq + Hash, V: Clone> MutableHashMapState<K, V> {
    #[inline]
    fn add_diff(&mut self, diff: MapDiff<K>) {
        self.pull_source.add_diff(diff);
    }

    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.hash_map.get(key)
    }

    #[inline]
    pub fn contains_key(&self, key: &K) -> bool {
        self.hash_map.contains_key(key)
    }

    #[inline]
    pub fn snapshot(&self) -> HashMap<K, V> {
        self.hash_map.clone()
    }

    pub fn replace<E>(&mut self, entries: E)
    where
        E: Iterator<Item = (K, V)>,
    {
        self.hash_map.clear();
        for (key, value) in entries {
            self.hash_map.insert(key, value);
        }
        self.add_diff(MapDiff::Replace {});
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let remember_k = k.clone();
        let result = self.hash_map.insert(k, v);

        self.add_diff(MapDiff::Insert { key: remember_k });
        return result;
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let result = self.hash_map.remove(k);
        if result.is_none() {
            return None;
        }

        self.add_diff(MapDiff::Remove { key: k.clone() });
        return result;
    }

    pub fn clear(&mut self) {
        if self.hash_map.is_empty() {
            return;
        }

        self.hash_map.clear();
        self.add_diff(MapDiff::Clear {})
    }
}
