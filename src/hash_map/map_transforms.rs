use super::event::{HashMapEvent, MapDiff};
use super::hash_map::{MutableHashMap, MutableHashMapState};
use crate::structural_signal::pull_source::PullSourceStructuralSignal;
use crate::structural_signal::transformer::StructuralSignalTransformer;
use core::hash::Hash;
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
                MapDiff::Insert { key } => {
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
                MapDiff::Insert { key } => {
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

    fn get_signal(&self) -> Self::OutputSignal {
        self.hash_map.as_signal()
    }
}
