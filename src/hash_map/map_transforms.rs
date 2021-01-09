use super::event::{HashMapEvent, MapDiff};
use super::hash_map::{MutableHashMap, MutableHashMapState};
use crate::structural_signal::pull_source::PullSourceStructuralSignal;
use crate::structural_signal::transformer::StructuralSignalTransformer;
use crate::vector::{MutableVector, VectorEvent};
use crate::ChannelStructuralSignal;
use core::hash::Hash;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::marker::PhantomData;

// ** MAP_VALUES ** //

pub struct MapHashMapTransformer<K, F, IV, OV>
where
    K: Hash + Eq + Clone,
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    hash_map: MutableHashMap<K, OV>,
    map_fn: F,
    input_type: PhantomData<IV>,
}

impl<K, F, IV, OV> MapHashMapTransformer<K, F, IV, OV>
where
    K: Hash + Eq + Clone,
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    pub(crate) fn new(map_fn: F) -> MapHashMapTransformer<K, F, IV, OV> {
        MapHashMapTransformer {
            hash_map: MutableHashMap::new(),
            map_fn: map_fn,
            input_type: PhantomData,
        }
    }
}

impl<K, F, IV, OV> StructuralSignalTransformer for MapHashMapTransformer<K, F, IV, OV>
where
    K: Hash + Eq + Clone,
    IV: Clone,
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    type InputEvent = HashMapEvent<K, IV>;
    type OutputSignal = PullSourceStructuralSignal<MutableHashMapState<K, OV>>;

    fn apply_event(&mut self, map_event: HashMapEvent<K, IV>) {
        let mut writer = self.hash_map.write();
        for diff in map_event.diffs {
            match diff {
                MapDiff::Replace {} => {
                    writer.replace(
                        map_event
                            .snapshot
                            .clone()
                            .into_iter()
                            .map(|(k, ov)| (k, (self.map_fn)(&ov))),
                    );
                }
                MapDiff::Insert { key } | MapDiff::Update { key } => {
                    let mapped_val = (self.map_fn)(map_event.snapshot.get(&key).unwrap());
                    writer.insert(key, mapped_val);
                }
                MapDiff::Remove { key } => {
                    writer.remove(&key);
                }
                MapDiff::Clear {} => {
                    writer.clear();
                }
            }
        }
    }

    #[inline]
    fn get_signal(&self) -> Self::OutputSignal {
        self.hash_map.as_signal()
    }
}

// ** FILTER ** //

pub struct FilterHashMapTransformer<K, V, F>
where
    K: Hash + Eq + Clone,
    V: Clone,
    F: Fn(&V) -> bool,
{
    hash_map: MutableHashMap<K, V>,
    predicate: F,
}

impl<K, V, F> FilterHashMapTransformer<K, V, F>
where
    K: Hash + Eq + Clone,
    V: Clone,
    F: Fn(&V) -> bool,
{
    pub(crate) fn new(predicate: F) -> FilterHashMapTransformer<K, V, F> {
        FilterHashMapTransformer {
            hash_map: MutableHashMap::new(),
            predicate: predicate,
        }
    }
}

impl<K, V, F> StructuralSignalTransformer for FilterHashMapTransformer<K, V, F>
where
    K: Hash + Eq + Clone,
    V: Clone,
    F: Fn(&V) -> bool,
{
    type InputEvent = HashMapEvent<K, V>;
    type OutputSignal = PullSourceStructuralSignal<MutableHashMapState<K, V>>;

    fn apply_event(&mut self, map_event: HashMapEvent<K, V>) {
        let mut writer = self.hash_map.write();
        for diff in map_event.diffs {
            match diff {
                MapDiff::Replace {} => {
                    writer.replace(
                        map_event
                            .snapshot
                            .clone()
                            .into_iter()
                            .filter(|(_k, v)| (self.predicate)(v)),
                    );
                }
                MapDiff::Insert { key } | MapDiff::Update { key } => {
                    let val = map_event.snapshot.get(&key).unwrap();
                    let passes_predicate = (self.predicate)(val);
                    if passes_predicate {
                        writer.insert(key, val.clone());
                        return;
                    }

                    let currently_exists = self.hash_map.read().contains_key(&key);
                    if currently_exists {
                        writer.remove(&key);
                    }
                }
                MapDiff::Remove { key } => {
                    writer.remove(&key);
                }
                MapDiff::Clear {} => {
                    writer.clear();
                }
            }
        }
    }

    #[inline]
    fn get_signal(&self) -> Self::OutputSignal {
        self.hash_map.as_signal()
    }
}

// ** ENTRIES ** //

pub struct EntriesHashMapTransformer<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    vector: MutableVector<(K, V)>,
}

impl<K, V> EntriesHashMapTransformer<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub(crate) fn new() -> EntriesHashMapTransformer<K, V> {
        EntriesHashMapTransformer {
            vector: MutableVector::new(),
        }
    }
}

#[inline]
fn hashed_key_sort<'r, K: Hash, V>(entry: &'r (K, V)) -> u64 {
    hash_key(&entry.0)
}

#[inline]
fn hash_key<K: Hash>(key: &K) -> u64 {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

impl<K, V> StructuralSignalTransformer for EntriesHashMapTransformer<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    type InputEvent = HashMapEvent<K, V>;
    type OutputSignal = ChannelStructuralSignal<VectorEvent<(K, V)>>;

    fn apply_event(&mut self, map_event: HashMapEvent<K, V>) {
        let mut writer = self.vector.write();
        for diff in map_event.diffs {
            match diff {
                MapDiff::Replace {} => {
                    let mut snapshot_vec = map_event
                        .snapshot
                        .clone()
                        .into_iter()
                        .collect::<Vec<(K, V)>>();
                    snapshot_vec.sort_by_key(hashed_key_sort);
                    writer.replace(snapshot_vec.into_iter());
                }
                MapDiff::Insert { key } => {
                    let key_hash = hash_key(&key);
                    let insert_at_index = writer.binary_search_by_key(&key_hash, hashed_key_sort);
                    let val = map_event.snapshot.get(&key).unwrap().clone();
                    match insert_at_index {
                        Result::Ok(_) => {
                            panic!("Found existing value for newly-inserted key in HashMap.entries()");
                        }
                        Result::Err(index) => {
                            writer.insert(index, (key, val));
                        }
                    }
                }
                MapDiff::Update { key } => {
                    let key_hash = hash_key(&key);
                    let insert_at_index = writer.binary_search_by_key(&key_hash, hashed_key_sort);
                    let val = map_event.snapshot.get(&key).unwrap().clone();
                    match insert_at_index {
                        Result::Ok(index) => {
                            writer.set(index, (key, val));
                        }
                        Result::Err(_) => {
                            panic!("Found no existing value for updated key in HashMap.entries()");
                        }
                    }
                }
                MapDiff::Remove { key } => {
                    let key_hash = hash_key(&key);
                    let remove_at_index = writer.binary_search_by_key(&key_hash, hashed_key_sort);
                    if let Result::Ok(index) = remove_at_index {
                        writer.remove(index);
                    }
                }
                MapDiff::Clear {} => {
                    writer.clear();
                }
            }
        }
    }

    #[inline]
    fn get_signal(&self) -> Self::OutputSignal {
        self.vector.as_signal()
    }
}
