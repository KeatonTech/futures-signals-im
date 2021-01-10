use crate::structural_signal::pull_source::PullableDiff;
use crate::structural_signal::structural_signal_ext::SnapshottableEvent;
use im::Vector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorDiff {
    Replace {},

    Insert { index: usize },

    Update { index: usize },

    Remove { index: usize },

    // TODO: Consider adding batch events, for example modeling
    // extending an array with all the items from another array.
    Clear {},
}

impl PullableDiff for VectorDiff {
    type KeyType = usize;

    fn get_key(&self) -> Option<&usize> {
        match self {
            VectorDiff::Insert { index }
            | VectorDiff::Update { index }
            | VectorDiff::Remove { index } => Some(index),
            VectorDiff::Replace {} | VectorDiff::Clear {} => None,
        }
    }

    fn set_key(&mut self, new_index: usize) {
        match self {
            VectorDiff::Insert { index }
            | VectorDiff::Remove { index }
            | VectorDiff::Update { index } => {
                *index = new_index;
            }
            VectorDiff::Replace {} | VectorDiff::Clear {} => {
                panic!("Cannot set key on non-keyed VectorDiff");
            }
        }
    }

    fn merge_with_previous(self, previous: VectorDiff) -> Option<VectorDiff> {
        if let VectorDiff::Insert { index } = previous {
            // Insert then Remove => Nothing
            if let VectorDiff::Remove { index: _ } = self {
                return None;
            }

            // Insert then Update => Insert
            if let VectorDiff::Update { index: _ } = self {
                return Some(VectorDiff::Insert { index });
            }
        }
        Some(self)
    }

    fn full_replace() -> VectorDiff {
        VectorDiff::Replace {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorEvent<T>
where
    T: Clone,
{
    pub snapshot: Vector<T>,
    pub diffs: Vec<VectorDiff>,
}

impl<T: Clone> SnapshottableEvent for VectorEvent<T> {
    type SnapshotType = Vector<T>;

    fn snapshot(&self) -> Self::SnapshotType {
        self.snapshot.clone()
    }
}
