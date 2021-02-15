use super::event::{VectorDiff, VectorEvent};
use crate::structural_signal::pull_source::{
    PullSourceHost, PullSourceStructuralSignal, StructrualSignalPullSource,
};
use im::Vector;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::cmp::max;
use std::convert::TryInto;
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
    pull_source: StructrualSignalPullSource<VectorDiff>,
}

impl<T: Clone> PullSourceHost for MutableVectorState<T> {
    type DiffType = VectorDiff;
    type EventType = VectorEvent<T>;

    fn get_pull_source<'a>(&'a mut self) -> &'a mut StructrualSignalPullSource<Self::DiffType> {
        &mut self.pull_source
    }

    fn make_event(&self, diffs: Vec<Self::DiffType>) -> Self::EventType {
        VectorEvent {
            snapshot: self.vector.clone(),
            diffs: diffs,
        }
    }
}

impl<T: Clone> MutableVectorState<T> {
    #[inline]
    fn add_diff(&mut self, diff: VectorDiff) {
        self.pull_source.add_diff(diff);
    }

    fn shift_diff_indices(&mut self, fulcrum: usize, delta: isize) {
        self.pull_source.update_keys(|index| {
            if *index >= fulcrum {
                max((*index as isize) + delta, 0) as usize
            } else {
                *index
            }
        })
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
                pull_source: StructrualSignalPullSource::new(),
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
                pull_source: StructrualSignalPullSource::new(),
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
    pub fn as_signal(&self) -> PullSourceStructuralSignal<MutableVectorState<T>> {
        PullSourceStructuralSignal::new(self.0.clone())
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
    pub fn as_signal(&self) -> PullSourceStructuralSignal<MutableVectorState<T>> {
        PullSourceStructuralSignal::new(self.0.clone())
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
        self.add_diff(VectorDiff::Replace {});
    }

    /// Replaces the value at a given index with a new value. Throws if the given
    /// index is not currently in the vector.
    pub fn set(&mut self, index: usize, value: T) -> T {
        let result = self.vector.set(index, value);

        self.add_diff(VectorDiff::Update {
            index,
            snapshot_index: index.try_into().unwrap(),
        });
        return result;
    }

    /// Inserts a new row into this vector at a given index. Throws if the given
    /// index is not currently in the vector.
    pub fn insert(&mut self, index: usize, value: T) {
        let result = self.vector.insert(index, value);

        self.shift_diff_indices(index, 1);
        self.add_diff(VectorDiff::Insert {
            index,
            snapshot_index: index.try_into().unwrap(),
        });
        return result;
    }

    /// Inserts a new item at the end of this Vector.
    pub fn push_back(&mut self, value: T) {
        let index = self.vector.len();
        let result = self.vector.push_back(value);

        self.add_diff(VectorDiff::Insert {
            index,
            snapshot_index: index.try_into().unwrap(),
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
        self.add_diff(VectorDiff::Remove {
            index,
            snapshot_index: index.try_into().unwrap(),
        });
        self.shift_diff_indices(index + 1, -1);
        return result;
    }

    /// Removes and returns the first value in this Vector, if the Vector is not empty.
    pub fn pop_front(&mut self) -> Option<T> {
        match self.vector.len() {
            0 => None,
            _ => Some(self.remove(0)),
        }
    }

    /// Removes and returns the last value in this Vector, if the Vector is not empty.
    pub fn pop_back(&mut self) -> Option<T> {
        match self.vector.len() {
            0 => None,
            _ => {
                let result = self.vector.pop_back().unwrap();
                self.add_diff(VectorDiff::Remove {
                    index: self.vector.len(),
                    snapshot_index: self.vector.len().try_into().unwrap(),
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
        self.add_diff(VectorDiff::Clear {});
    }
}

impl<T: Clone> Deref for MutableVectorState<T> {
    type Target = Vector<T>;

    fn deref(&self) -> &Self::Target {
        &self.vector
    }
}
