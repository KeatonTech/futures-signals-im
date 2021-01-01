use super::event::{HashMapEvent, MapDiff};
use super::map_transforms::{
    FilterHashMapTransformer, MapHashMapTransformer, TransformedMutableHashMap,
};
use crate::StructuralSignal;
use core::hash::Hash;
use futures_executor::block_on;
use futures_signals::signal::Signal;
use futures_util::future::poll_fn;
use im::HashMap;
use pin_project::pin_project;
use pin_utils::pin_mut;
use std::pin::Pin;
use std::task::{Context, Poll};
#[pin_project(project = SignalHashMapKeyWatcherProj)]
pub struct SignalHashMapKeyWatcher<K, V, S>
where
    K: Clone + Eq + Hash,
    V: Clone,
    S: StructuralSignal<Item=HashMapEvent<K, V>>,
{
    #[pin]
    signal: S,
    key: K,
}

impl<K, V, S> Signal for SignalHashMapKeyWatcher<K, V, S>
where
    K: Clone + Eq + Hash,
    V: Clone,
    S: StructuralSignal<Item=HashMapEvent<K, V>>,
{
    type Item = Option<V>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Option<V>>> {
        let SignalHashMapKeyWatcherProj { signal, key } = self.project();
        let match_key = key;

        match signal.poll_change(cx) {
            Poll::Ready(Some(hash_map_event)) => match hash_map_event.diff {
                MapDiff::Replace {} => Poll::Ready(Some(
                    hash_map_event.snapshot.get(match_key).map(|v| v.clone()),
                )),
                MapDiff::Insert { key } => {
                    if key == *match_key {
                        Poll::Ready(Some(
                            hash_map_event.snapshot.get(match_key).map(|v| v.clone()),
                        ))
                    } else {
                        Poll::Pending
                    }
                }
                MapDiff::Remove { key } => {
                    if key == *match_key {
                        Poll::Ready(Some(None))
                    } else {
                        Poll::Pending
                    }
                }
                MapDiff::Clear {} => Poll::Ready(Some(None)),
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub trait SignalHashMapExt: StructuralSignal
where
    Self: Sized,
{
    type Key: Clone + Eq + Hash;
    type Value: Clone;
    type SelfType: StructuralSignal<Item=HashMapEvent<Self::Key, Self::Value>>;
        
    /// Returns a Signal that tracks the value of a particular key in the Map.
    fn get_signal_for_key(
        self,
        key: Self::Key,
    ) -> SignalHashMapKeyWatcher<Self::Key, Self::Value, Self::SelfType>;

    /// Returns a version of this signal where every value in the map has been run
    /// through a transformer function.
    ///
    /// ```
    /// use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
    /// use im::hashmap;
    ///
    /// let input_map = MutableHashMap::<u8, u8>::new();
    /// input_map.write().insert(1, 1);
    ///
    /// let multiplied = input_map.as_signal().map_values(|v| v * 2);
    /// input_map.write().insert(2, 2);
    ///
    /// let multiplied_map = multiplied.into_map_sync().unwrap();
    /// assert_eq!(multiplied_map, hashmap!{1 => 2, 2 => 4});
    /// ```
    fn map_values<OV, F>(
        self,
        map_fn: F,
    ) -> TransformedMutableHashMap<Self::SelfType, MapHashMapTransformer<Self::Key, F, Self::Value, OV>>
    where
        OV: Clone,
        Self::Value: Clone,
        F: Fn(&Self::Value) -> OV;

    /// Returns a version of this signal that includes only map entries that pass a predicate test.
    ///
    /// ```
    /// use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
    /// use im::hashmap;
    ///
    /// let input_map = MutableHashMap::<u8, u8>::new();
    /// input_map.write().insert(1, 1);
    /// input_map.write().insert(2, 1);
    /// input_map.write().insert(3, 2);
    /// input_map.write().insert(4, 3);
    ///
    /// let odds_only = input_map.as_signal().filter(|v| v % 2 == 1);
    ///
    /// let odds_only_map = odds_only.into_map_sync().unwrap();
    /// assert_eq!(odds_only_map, hashmap!{1 => 1, 2 => 1, 4 => 3});
    /// ```
    fn filter<F>(
        self,
        predicate: F,
    ) -> TransformedMutableHashMap<Self::SelfType, FilterHashMapTransformer<Self::Key, Self::Value, F>>
    where
        Self::Value: Clone,
        F: Fn(&Self::Value) -> bool;

    /// Retrieves the current value of the Signal as an IM HashMap.
    ///
    /// ```
    /// use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
    /// use im::hashmap;
    ///
    /// let input_map = MutableHashMap::<u8, u8>::new();
    /// input_map.write().insert(1, 1);
    ///
    /// let signal = input_map.as_signal();
    /// let current_val = signal.into_map_sync().unwrap();
    /// assert_eq!(current_val, hashmap!{1 => 1});
    /// ```
    fn into_map_sync(self) -> Option<HashMap<Self::Key, Self::Value>>;
}

impl<K, V, I> SignalHashMapExt for I
where
    I: StructuralSignal<Item=HashMapEvent<K, V>>,
    K: Clone + Eq + Hash,
    V: Clone,
{
    type Key = K;
    type Value = V;
    type SelfType = I;

    fn get_signal_for_key(
        self,
        key: Self::Key,
    ) -> SignalHashMapKeyWatcher<Self::Key, Self::Value, Self>
    where
        Self: Sized,
    {
        SignalHashMapKeyWatcher { signal: self, key }
    }

    fn map_values<OV, F>(
        self,
        map_fn: F,
    ) -> TransformedMutableHashMap<Self, MapHashMapTransformer<Self::Key, F, Self::Value, OV>>
    where
        OV: Clone,
        Self::Value: Clone,
        F: Fn(&Self::Value) -> OV,
        Self: Sized,
    {
        TransformedMutableHashMap::new(self, MapHashMapTransformer::new(map_fn))
    }

    fn filter<F>(
        self,
        predicate: F,
    ) -> TransformedMutableHashMap<Self, FilterHashMapTransformer<Self::Key, Self::Value, F>>
    where
        Self::Value: Clone,
        F: Fn(&Self::Value) -> bool,
    {
        TransformedMutableHashMap::new(self, FilterHashMapTransformer::new(predicate))
    }

    fn into_map_sync(self) -> Option<HashMap<K, V>> {
        let signal = self;
        pin_mut!(signal);
        let poll_result = block_on(poll_fn(|cx| {
            let mut prev_event: Option<HashMapEvent<K, V>> = None;
            let maybe_event = loop {
                match Pin::as_mut(&mut signal).poll_change(cx) {
                    Poll::Ready(Some(event)) => {
                        prev_event = Some(event);
                        continue;
                    }
                    Poll::Ready(None) | Poll::Pending => {
                        break prev_event;
                    }
                }
            };
            match maybe_event {
                Some(event) => Poll::Ready(event.snapshot),
                _ => Poll::Pending,
            }
        }));
        return poll_result.into();
    }
}
