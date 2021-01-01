use super::event::{HashMapEvent, MapDiff};
use super::hash_map::{MutableHashMap, MutableHashMapSignal};
use crate::StructuralSignal;
use core::hash::Hash;
use pin_project::pin_project;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait MutableHashMapTransformer {
    type InputKey: Eq + Hash + Clone;
    type InputValue: Clone;
    type OutputKey: Eq + Hash + Clone;
    type OutputValue: Clone;

    fn apply_event(&mut self, map_event: HashMapEvent<Self::InputKey, Self::InputValue>);
    fn get_transformed_hash_map<'a>(
        &'a self,
    ) -> &'a MutableHashMap<Self::OutputKey, Self::OutputValue>;
}

#[pin_project(project = TransformedMutableHashMapProj)]
pub struct TransformedMutableHashMap<IS, T>
where
    IS: StructuralSignal<Item=HashMapEvent<T::InputKey, T::InputValue>>,
    T: MutableHashMapTransformer,
{
    #[pin]
    input_signal: IS,
    #[pin]
    transformed_signal: MutableHashMapSignal<T::OutputKey, T::OutputValue>,
    transformer: T,
}

impl<IS, T> TransformedMutableHashMap<IS, T>
where
    IS: StructuralSignal<Item=HashMapEvent<T::InputKey, T::InputValue>>,
    T: MutableHashMapTransformer,
{
    pub(crate) fn new(input_signal: IS, transformer: T) -> TransformedMutableHashMap<IS, T> {
        let transformed_signal = transformer.get_transformed_hash_map().as_signal();
        TransformedMutableHashMap {
            input_signal,
            transformed_signal,
            transformer,
        }
    }
}

impl<IS, T> StructuralSignal for TransformedMutableHashMap<IS, T>
where
    T: MutableHashMapTransformer,
    IS: StructuralSignal<Item=HashMapEvent<T::InputKey, T::InputValue>>,
{
    type Item = HashMapEvent<T::OutputKey, T::OutputValue>;

    // TODO should this inline ?
    #[inline]
    fn poll_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<HashMapEvent<T::OutputKey, T::OutputValue>>> {
        let TransformedMutableHashMapProj {
            mut input_signal,
            transformed_signal,
            transformer,
        } = self.project();

        loop {
            let input_poll = input_signal.as_mut().poll_change(cx);
            match input_poll {
                Poll::Ready(Some(event)) => {
                    transformer.apply_event(event);
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    break;
                }
            }
        }

        return transformed_signal.poll_change(cx);
    }
}


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

impl<K, F, IV, OV> MutableHashMapTransformer for MapHashMapTransformer<K, F, IV, OV>
where
    K: Hash + Eq + Clone,
    IV: Clone,
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    type InputKey = K;
    type InputValue = IV;
    type OutputKey = K;
    type OutputValue = OV;

    fn apply_event(&mut self, map_event: HashMapEvent<Self::InputKey, Self::InputValue>) {
        let mut writer = self.hash_map.write();
        match map_event.diff {
            MapDiff::Replace {} => {
                writer.replace(
                    map_event
                        .snapshot
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

    fn get_transformed_hash_map<'a>(
        &'a self,
    ) -> &'a MutableHashMap<Self::OutputKey, Self::OutputValue> {
        &self.hash_map
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

impl<K, V, F> MutableHashMapTransformer for FilterHashMapTransformer<K, V, F>
where
    K: Hash + Eq + Clone,
    V: Clone,
    F: Fn(&V) -> bool,
{
    type InputKey = K;
    type InputValue = V;
    type OutputKey = K;
    type OutputValue = V;

    fn apply_event(&mut self, map_event: HashMapEvent<Self::InputKey, Self::InputValue>) {
        let mut writer = self.hash_map.write();
        match map_event.diff {
            MapDiff::Replace {} => {
                writer.replace(
                    map_event
                        .snapshot
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
                if  currently_exists {
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

    fn get_transformed_hash_map<'a>(
        &'a self,
    ) -> &'a MutableHashMap<Self::OutputKey, Self::OutputValue> {
        &self.hash_map
    }
}
