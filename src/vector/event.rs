use crate::structural_signal::pull_source::DiffMergeResult;
use crate::structural_signal::pull_source::PullableDiff;
use crate::structural_signal::structural_signal_ext::SnapshottableEvent;
use im::Vector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorDiff {
    Replace {},

    Insert { index: usize, snapshot_index: usize },

    Update { index: usize, snapshot_index: usize },

    Remove { index: usize, snapshot_index: usize },

    // TODO: Consider adding batch events, for example modeling
    // extending an array with all the items from another array.
    Clear {},
}

impl PullableDiff for VectorDiff {
    type KeyType = usize;

    fn get_key(&self) -> Option<&usize> {
        match self {
            VectorDiff::Insert {
                index,
                snapshot_index: _,
            }
            | VectorDiff::Update {
                index,
                snapshot_index: _,
            }
            | VectorDiff::Remove {
                index,
                snapshot_index: _,
            } => Some(index),
            VectorDiff::Replace {} | VectorDiff::Clear {} => None,
        }
    }

    fn get_snapshot_key(&self) -> Option<&usize> {
        match self {
            VectorDiff::Insert {
                index: _,
                snapshot_index,
            }
            | VectorDiff::Update {
                index: _,
                snapshot_index,
            }
            | VectorDiff::Remove {
                index: _,
                snapshot_index,
            } => Some(snapshot_index),
            VectorDiff::Replace {} | VectorDiff::Clear {} => None,
        }
    }

    fn set_key(&mut self, new_index: usize) {
        match self {
            VectorDiff::Insert {
                index,
                snapshot_index: _,
            }
            | VectorDiff::Update {
                index,
                snapshot_index: _,
            }
            | VectorDiff::Remove {
                index,
                snapshot_index: _,
            } => {
                *index = new_index;
            }
            _ => {
                panic!("Cannot set key on non-keyed VectorDiff");
            }
        }
    }

    fn set_snapshot_key(&mut self, new_index: usize) {
        match self {
            VectorDiff::Insert {
                index: _,
                snapshot_index,
            }
            | VectorDiff::Update {
                index: _,
                snapshot_index,
            }
            | VectorDiff::Remove {
                index: _,
                snapshot_index,
            } => {
                *snapshot_index = new_index;
            }
            _ => {
                panic!("Cannot set key on non-keyed VectorDiff");
            }
        }
    }

    fn merge_with_previous(&self, previous: &VectorDiff) -> DiffMergeResult<VectorDiff> {
        if let &VectorDiff::Insert {
            index: _,
            snapshot_index,
        } = previous
        {
            // Insert then Remove => Nothing
            if let VectorDiff::Remove {
                index: _,
                snapshot_index: _,
            } = self
            {
                let pivot = snapshot_index;
                return DiffMergeResult::discard_both_and_reindex(move |i: &usize, si: &usize| {
                    if *si > pivot {
                        *i - 1
                    } else {
                        *i
                    }
                });
            }

            // Insert then Update => Insert (unchanged)
            if let VectorDiff::Update {
                index: _,
                snapshot_index: _,
            } = self
            {
                return DiffMergeResult::<VectorDiff>::ignore();
            }

            // Two inserts on the same index should never happen
            if let VectorDiff::Insert {
                index: _,
                snapshot_index: _,
            } = self
            {
                panic!("Found two inserts on the same index. The second should be an update.")
            }
        } else if let VectorDiff::Remove {
            index: _,
            snapshot_index: _,
        } = previous
        {
            // Remove then Insert => Update
            if let &VectorDiff::Insert {
                index,
                snapshot_index,
            } = self
            {
                let pivot = snapshot_index;
                return DiffMergeResult::merge_and_reindex(
                    VectorDiff::Update {
                        index: index,
                        snapshot_index: snapshot_index,
                    },
                    move |i: &usize, si: &usize| {
                        if *si > pivot {
                            *i + 1
                        } else {
                            *i
                        }
                    },
                );
            }

            return DiffMergeResult::keep_both();
        }
        return DiffMergeResult::replace();
    }

    fn full_replace() -> VectorDiff {
        VectorDiff::Replace {}
    }
}

impl VectorDiff {
    pub fn get_value_from_snapshot<'a, C>(&self, from_snapshot: &'a C) -> Option<&'a C::Output>
    where
        C: std::ops::Index<usize>,
        C::Output: Sized,
    {
        match self {
            VectorDiff::Insert {
                index: _,
                snapshot_index,
            }
            | VectorDiff::Update {
                index: _,
                snapshot_index,
            } => Some(&from_snapshot[*snapshot_index]),
            _ => None,
        }
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
