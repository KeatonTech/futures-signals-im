use super::event::HashMapEvent;
use super::map_transforms::{
    EntriesHashMapTransformer, FilterHashMapTransformer, MapHashMapTransformer,
};
use crate::structural_signal::pull_source::PullableDiff;
use crate::structural_signal::transformer::TransformedStructuralSignal;
use crate::StructuralSignal;
use core::hash::Hash;
use futures_signals::signal::Signal;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

#[pin_project(project = SignalHashMapKeyWatcherProj)]
pub struct SignalHashMapKeyWatcher<K, V, S>
where
    K: Clone + Eq + Hash,
    V: Clone,
    S: StructuralSignal<Item = HashMapEvent<K, V>>,
{
    #[pin]
    signal: S,
    key: K,
}

impl<K, V, S> Signal for SignalHashMapKeyWatcher<K, V, S>
where
    K: Clone + Eq + Hash,
    V: Clone,
    S: StructuralSignal<Item = HashMapEvent<K, V>>,
{
    type Item = Option<V>;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Option<V>>> {
        let SignalHashMapKeyWatcherProj {
            signal,
            key: local_key,
        } = self.project();

        match signal.poll_change(cx) {
            Poll::Ready(Some(hash_map_event)) => {
                let has_key_event = hash_map_event
                    .diffs
                    .iter()
                    .find(|diff| match diff.get_key() {
                        Some(key) => *key == *local_key,
                        None => false,
                    })
                    .is_some();
                if has_key_event {
                    Poll::Ready(Some(
                        hash_map_event.snapshot.get(local_key).map(|v| v.clone()),
                    ))
                } else {
                    let has_global_event = hash_map_event
                        .diffs
                        .iter()
                        .find(|diff| match diff.get_key() {
                            Some(_key) => false,
                            None => true,
                        })
                        .is_some();
                    if has_global_event {
                        Poll::Ready(Some(
                            hash_map_event.snapshot.get(local_key).map(|v| v.clone()),
                        ))
                    } else {
                        Poll::Pending
                    }
                }
            }
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
    type SelfType: StructuralSignal<Item = HashMapEvent<Self::Key, Self::Value>>;
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
    /// use signals_im::StructuralSignalExt;
    /// use im::hashmap;
    ///
    /// let input_map = MutableHashMap::<u8, u8>::new();
    /// input_map.write().insert(1, 1);
    ///
    /// let multiplied = input_map.as_signal().map_values(|v| v * 2);
    /// input_map.write().insert(2, 2);
    ///
    /// let multiplied_map = multiplied.snapshot().unwrap();
    /// assert_eq!(multiplied_map, hashmap!{1 => 2, 2 => 4});
    /// ```
    fn map_values<OV, F>(
        self,
        map_fn: F,
    ) -> TransformedStructuralSignal<
        Self::SelfType,
        <Self::SelfType as StructuralSignal>::Item,
        MapHashMapTransformer<Self::Key, F, Self::Value, OV>,
    >
    where
        OV: Clone,
        Self::Value: Clone,
        F: Fn(&Self::Value) -> OV;

    /// Returns a version of this signal that includes only map entries that pass a predicate test.
    ///
    /// ```
    /// use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
    /// use signals_im::StructuralSignalExt;
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
    /// let odds_only_map = odds_only.snapshot().unwrap();
    /// assert_eq!(odds_only_map, hashmap!{1 => 1, 2 => 1, 4 => 3});
    /// ```
    fn filter<F>(
        self,
        predicate: F,
    ) -> TransformedStructuralSignal<
        Self::SelfType,
        <Self::SelfType as StructuralSignal>::Item,
        FilterHashMapTransformer<Self::Key, Self::Value, F>,
    >
    where
        Self::Value: Clone,
        F: Fn(&Self::Value) -> bool;

    /// Returns a version of this signal that includes only map entries that pass a predicate test.
    ///
    /// ```
    /// use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
    /// use signals_im::StructuralSignalExt;
    /// use im::vector;
    ///
    /// let input_map = MutableHashMap::<u8, u8>::new();
    /// input_map.write().insert(1, 1);
    /// input_map.write().insert(2, 1);
    /// input_map.write().insert(3, 2);
    /// input_map.write().insert(4, 3);
    ///
    /// let entries_signal = input_map.as_signal().entries();
    ///
    /// let entries = entries_signal.snapshot().unwrap();
    /// assert_eq!(entries, vector![(1, 1), (3, 2), (2, 1), (4, 3)]);
    /// ```
    fn entries(
        self,
    ) -> TransformedStructuralSignal<
        Self::SelfType,
        <Self::SelfType as StructuralSignal>::Item,
        EntriesHashMapTransformer<Self::Key, Self::Value>,
    >
    where
        Self::Value: Clone;
}

impl<K, V, I> SignalHashMapExt for I
where
    I: StructuralSignal<Item = HashMapEvent<K, V>>,
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
    ) -> TransformedStructuralSignal<
        Self,
        Self::Item,
        MapHashMapTransformer<Self::Key, F, Self::Value, OV>,
    >
    where
        OV: Clone,
        Self::Value: Clone,
        F: Fn(&Self::Value) -> OV,
        Self: Sized,
    {
        TransformedStructuralSignal::new(self, MapHashMapTransformer::new(map_fn))
    }

    fn filter<F>(
        self,
        predicate: F,
    ) -> TransformedStructuralSignal<
        Self,
        Self::Item,
        FilterHashMapTransformer<Self::Key, Self::Value, F>,
    >
    where
        Self::Value: Clone,
        F: Fn(&Self::Value) -> bool,
    {
        TransformedStructuralSignal::new(self, FilterHashMapTransformer::new(predicate))
    }

    fn entries(
        self,
    ) -> TransformedStructuralSignal<
        Self::SelfType,
        <Self::SelfType as StructuralSignal>::Item,
        EntriesHashMapTransformer<Self::Key, Self::Value>,
    >
    where
        Self::Value: Clone,
    {
        TransformedStructuralSignal::new(self, EntriesHashMapTransformer::new())
    }
}
