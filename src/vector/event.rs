use im::Vector;
use crate::structural_signal_ext::SnapshottableEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorIndex {
    Index(usize),
    
    /// Shortcut to the last index in the list. Shortcut to allowing
    /// PUSH and POP operations.
    LastIndex,
} 

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorDiff {
    Replace {},

    Insert {
        index: VectorIndex,
    },

    Update {
        index: VectorIndex,
    },

    Remove {
        index: VectorIndex,
    },

    // TODO: Consider adding batch events, for example modeling
    // extending an array with all the items from another array.

    Clear {},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorEvent<T> where T: Clone {
    pub snapshot: Vector<T>,
    pub diff: VectorDiff
}

impl<T: Clone> SnapshottableEvent for VectorEvent<T> {
    type SnapshotType = Vector<T>;

    fn snapshot(&self) -> Self::SnapshotType {
        self.snapshot.clone()
    }
}