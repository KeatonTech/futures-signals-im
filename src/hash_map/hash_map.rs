use super::event::{HashMapEvent, MapDiff};
use super::signal::SignalHashMap;
use futures::channel::mpsc;
use futures_util::StreamExt;
use im::HashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::iter::Iterator;

// The senders vec will be cleaned up once at least this many senders
// have been closed.
static CHANNEL_CLEANUP_MIN_COUNT: i16 = 20;

/// The internal state of a MutableHashMap or MutableHashMapReader. All
/// clones (and readonly clones) will share this same instance.
///
/// The only difference between `MutableHashMap` and `MutableHashMapReader`
/// is that the latter provides no way to get a mutable reference to this
/// struct and therefore cannot perform any modification operation.
#[derive(Debug)]
pub struct MutableHashMapState<K: Clone + Eq + Hash, V: Clone> {
    hash_map: HashMap<K, V>,
    senders: RwLock<Vec<Option<mpsc::UnboundedSender<HashMapEvent<K, V>>>>>,
}

impl<K: Clone + Eq + Hash, V: Clone> MutableHashMapState<K, V> {
    fn notify(&self, event: HashMapEvent<K, V>) {
        // Tracks how many streams in the senders vec have expired. If this
        // exceeds a certain threshold, clean up the senders vec to improve
        // performance and reduce memory use.
        let mut expired_count = 0;

        for maybe_sender in self.senders.write().iter_mut() {
            if let Some(sender) = maybe_sender {
                if sender.is_closed() {
                    maybe_sender.take();
                    expired_count += 1;
                } else {
                    sender.unbounded_send(event.clone()).unwrap();
                }
            } else {
                expired_count += 1;
            }
        }

        if expired_count > CHANNEL_CLEANUP_MIN_COUNT {
            self.cleanup_expired_channels();
        }
    }

    fn cleanup_expired_channels(&self) {
        self.senders
            .write()
            .retain(|maybe_sender| maybe_sender.is_some());
    }
}

#[doc(hidden)]
pub struct MutableHashMapSignal<K: Clone + Eq + Hash, V: Clone> {
    receiver: mpsc::UnboundedReceiver<HashMapEvent<K, V>>,
}

impl<K: Clone + Eq + Hash, V: Clone> MutableHashMapSignal<K, V> {
    pub fn new(
        receiver: mpsc::UnboundedReceiver<HashMapEvent<K, V>>,
    ) -> MutableHashMapSignal<K, V> {
        MutableHashMapSignal { receiver }
    }
}

impl<K: Clone + Eq + Hash, V: Clone> Unpin for MutableHashMapSignal<K, V> {}

impl<K: Clone + Eq + Hash, V: Clone> SignalHashMap for MutableHashMapSignal<K, V> {
    type Key = K;
    type Value = V;

    #[inline]
    fn poll_map_change(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<HashMapEvent<Self::Key, Self::Value>>> {
        self.receiver.poll_next_unpin(cx)
    }
}

/// A HashMap that can be observed as it changes over time.
///
/// Internally this is an Arc, so calling `.clone()` on it will duplicate
/// the reference but not the data. This allows multiple objects or threads
/// to access and modify this object at once (protected by an RwLock). Use the
/// `.duplicate()` method to create a separate object that will not be affected
/// by future updates.
///
/// This structure is backed by `im.HashMap` and so requires that keys and values
/// are clonable. The backing structure is optimized to clone only when necessary.
pub struct MutableHashMap<K: Clone + Eq + Hash, V: Clone>(Arc<RwLock<MutableHashMapState<K, V>>>);

impl<K: Clone + Eq + Hash, V: Clone> Clone for MutableHashMap<K, V> {
    fn clone(&self) -> Self {
        MutableHashMap {
            0: Arc::new(RwLock::new(MutableHashMapState {
                hash_map: self.0.read().hash_map.clone(),
                senders: RwLock::new(vec![]),
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
                senders: RwLock::new(vec![]),
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
    pub fn as_signal(&self) -> MutableHashMapSignal<K, V> {
        self.0.read().as_signal()
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
    pub fn as_signal(&self) -> MutableHashMapSignal<K, V> {
        self.0.read().as_signal()
    }
}

impl<K: Clone + Eq + Hash, V: Clone> MutableHashMapState<K, V> {
    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.hash_map.get(key)
    }

    #[inline]
    pub fn contains_key(&self, key: &K) -> bool {
        self.hash_map.contains_key(key)
    }

    pub fn as_signal(&self) -> MutableHashMapSignal<K, V> {
        let (sender, receiver) = mpsc::unbounded();
        if !self.hash_map.is_empty() {
            sender
                .unbounded_send(HashMapEvent {
                    snapshot: self.hash_map.clone(),
                    diff: MapDiff::Replace {},
                })
                .unwrap();
        }

        self.senders.write().push(Some(sender));
        MutableHashMapSignal::new(receiver)
    }

    #[inline]
    pub fn snapshot(&self) -> HashMap<K, V> {
        self.hash_map.clone()
    }

    pub fn replace<E>(&mut self, entries: E) where E: Iterator<Item=(K, V)> {
        self.hash_map.clear();
        for (key, value) in entries {
            self.hash_map.insert(key, value);
        }
        self.notify(HashMapEvent {
            snapshot: self.snapshot(),
            diff: MapDiff::Replace {},
        });
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let remember_k = k.clone();
        let result = self.hash_map.insert(k, v);

        self.notify(HashMapEvent {
            snapshot: self.snapshot(),
            diff: MapDiff::Insert { key: remember_k },
        });
        return result;
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let result = self.hash_map.remove(k);
        if result.is_none() {
            return None;
        }

        self.notify(HashMapEvent {
            snapshot: self.snapshot(),
            diff: MapDiff::Remove { key: k.clone() },
        });
        return result;
    }

    pub fn clear(&mut self) {
        if self.hash_map.is_empty() {
            return;
        }

        self.hash_map.clear();
        self.notify(HashMapEvent {
            snapshot: self.snapshot(),
            diff: MapDiff::Clear {},
        });
    }
}
