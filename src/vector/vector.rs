use super::event::{VectorDiff, VectorEvent, VectorIndex};
use crate::util::notify_senders;
use crate::ChannelStructuralSignal;
use futures::channel::mpsc;
use im::Vector;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::iter::FromIterator;
use std::iter::Iterator;
use std::ops::{Deref, Index};
use std::slice::SliceIndex;
use std::sync::Arc;

/// The internal state of a MutableVector or MutableVectorReader. All
/// clones (and readonly clones) will share this same instance.
///
/// The only difference between `MutableVector` and `MutableVectorReader`
/// is that the latter prov ides no way to get a mutable reference to this
/// struct and therefore cannot perform any modification operation.
#[derive(Debug)]
pub struct MutableVectorState<T: Clone> {
    vector: Vector<T>,
    senders: RwLock<Vec<Option<mpsc::UnboundedSender<VectorEvent<T>>>>>,
}

impl<T: Clone> MutableVectorState<T> {
    fn notify(&self, event: VectorEvent<T>) {
        notify_senders(event, self.senders.write());
    }
}

/// A Vector that can be observed as it changes over time.
///
/// This structure is backed by `im.Vector` and so requires that values are clonable.
/// The backing structure is optimized to clone only when necessary.
pub struct MutableVector<T: Clone>(Arc<RwLock<MutableVectorState<T>>>);

impl<T: Clone> Clone for MutableVector<T> {
    fn clone(&self) -> Self {
        MutableVector {
            0: Arc::new(RwLock::new(MutableVectorState {
                vector: self.0.read().vector.clone(),
                senders: RwLock::new(vec![]),
            })),
        }
    }
}

impl<T: Clone> MutableVector<T> {
    /// Returns a readonly view into the underlying state that can be used
    /// to read values from the Vector.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<MutableVectorState<T>> {
        self.0.read()
    }

    /// Returns a writer into the underlying state that can be used to
    /// modify the Vector.
    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<MutableVectorState<T>> {
        self.0.write()
    }

    pub fn new() -> Self {
        MutableVector {
            0: Arc::new(RwLock::new(MutableVectorState {
                vector: Vector::new(),
                senders: RwLock::new(vec![]),
            })),
        }
    }

    /// Creates a read-only view into this data structure. This Reader object
    /// can lookup items in this map at any time, but cannot modify it.
    /// Readers can be cloned. Note that Readers can hold a ReadLock which can
    /// block MutableVector::write() calls, so use them with care.
    #[inline]
    pub fn reader(&self) -> MutableVectorReader<T> {
        MutableVectorReader { 0: self.0.clone() }
    }

    /// Creates a signal that tracks the value of this Vector. Signals can be directly
    /// used for UI, or can be transformed (TBD).
    #[inline]
    pub fn as_signal(&self) -> ChannelStructuralSignal<VectorEvent<T>> {
        self.0.read().as_signal()
    }
}

/// A read-only view into a MutableVector.
pub struct MutableVectorReader<T: Clone>(Arc<RwLock<MutableVectorState<T>>>);

impl<T: Clone> Clone for MutableVectorReader<T> {
    #[inline]
    fn clone(&self) -> Self {
        MutableVectorReader { 0: self.0.clone() }
    }
}

impl<T: Clone> MutableVectorReader<T> {
    /// Returns a readonly view into the underlying state that can be used
    /// to read values from the Vector.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<MutableVectorState<T>> {
        self.0.read()
    }

    /// Creates a signal that tracks the value of this Vector. Signals can be directly
    /// used for UI, or can be transformed (TBD).
    #[inline]
    pub fn as_signal(&self) -> ChannelStructuralSignal<VectorEvent<T>> {
        self.0.read().as_signal()
    }
}

impl<T: Clone, I> Index<I> for MutableVectorState<T>
where
    I: SliceIndex<[T]>,
    I: Into<usize>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(&self.vector, index.into())
    }
}

impl<T: Clone> MutableVectorState<T> {
    pub fn len(&self) -> usize {
        self.vector.len()
    }

    /// Gets the current value at a given index. Throws if the given
    /// index is not currently in the vector.
    pub fn get(&self, index: usize) -> &T {
        &self[index]
    }
    
    /// Creates a signal that tracks the value of this Vector. Signals can be directly
    /// used for UI, or can be transformed (TBD).
    pub fn as_signal(&self) -> ChannelStructuralSignal<VectorEvent<T>> {
        let (sender, receiver) = mpsc::unbounded();
        if !self.vector.is_empty() {
            sender
                .unbounded_send(VectorEvent {
                    snapshot: self.vector.clone(),
                    diff: VectorDiff::Replace {},
                })
                .unwrap();
        }

        self.senders.write().push(Some(sender));
        ChannelStructuralSignal::new(receiver)
    }

    /// Creates an immutable snapshot of the current state of this Vector. This
    /// operation is fairly cheap thanks to the backing Immutable data structure.
    /// Future changes to this MutableVector will not alter the snapshot.
    #[inline]
    pub fn snapshot(&self) -> Vector<T> {
        self.vector.clone()
    }

    /// Replaces the entire contents of this Vector with new entries. All existing
    /// data will be cleared.
    pub fn replace<E>(&mut self, entries: E)
    where
        E: Iterator<Item = T>,
    {
        self.vector.clear();
        self.vector.append(Vector::from_iter(entries));
        self.notify(VectorEvent {
            snapshot: self.snapshot(),
            diff: VectorDiff::Replace {},
        });
    }

    /// Replaces the value at a given index with a new value. Throws if the given
    /// index is not currently in the vector.
    pub fn set(&mut self, index: usize, value: T) -> T {
        let result = self.vector.set(index, value);

        self.notify(VectorEvent {
            snapshot: self.snapshot(),
            diff: VectorDiff::Update {
                index: VectorIndex::Index(index),
            },
        });
        return result;
    }

    /// Inserts a new row into this vector at a given index. Throws if the given
    /// index is not currently in the vector.
    pub fn insert(&mut self, index: usize, value: T) {
        let result = self.vector.insert(index, value);

        self.notify(VectorEvent {
            snapshot: self.snapshot(),
            diff: VectorDiff::Insert {
                index: VectorIndex::Index(index),
            },
        });
        return result;
    }

    /// Inserts a new item at the end of this Vector.
    pub fn push_back(&mut self, value: T) {
        let result = self.vector.push_back(value);

        self.notify(VectorEvent {
            snapshot: self.snapshot(),
            diff: VectorDiff::Insert {
                index: VectorIndex::LastIndex,
            },
        });
        return result;
    }

    /// Inserts a new item at the front of this Vector.
    pub fn push_front(&mut self, value: T) {
        self.insert(0, value)
    }

    /// Removes and returns the item at a given index. Throws if the given
    /// index is not currently in the vector.
    pub fn remove(&mut self, index: usize) -> T {
        let result = self.vector.remove(index);
        self.notify(VectorEvent {
            snapshot: self.snapshot(),
            diff: VectorDiff::Remove { index: VectorIndex::Index(index) },
        });
        return result;
    }

    /// Removes and returns the first value in this Vector, if the Vector is not empty.
    pub fn pop_front(&mut self) -> Option<T> {
        match self.vector.len() {
            0 => None,
            _ => Some(self.remove(0))
        }
    }

    /// Removes and returns the last value in this Vector, if the Vector is not empty.
    pub fn pop_back(&mut self) -> Option<T> {
        match self.vector.len() {
            0 => None,
            _ => {
                let result = self.vector.pop_back().unwrap();
                self.notify(VectorEvent {
                    snapshot: self.snapshot(),
                    diff: VectorDiff::Remove { index: VectorIndex::LastIndex },
                });
                Some(result)
            }
        }
    }

    /// Removes every value in this Vector.
    pub fn clear(&mut self) {
        if self.vector.is_empty() {
            return;
        }

        self.vector.clear();
        self.notify(VectorEvent {
            snapshot: self.snapshot(),
            diff: VectorDiff::Clear {},
        });
    }
}

impl<T: Clone> Deref for MutableVectorState<T> {
    type Target = Vector<T>;

    fn deref(&self) -> &Self::Target {
        &self.vector
    }
}