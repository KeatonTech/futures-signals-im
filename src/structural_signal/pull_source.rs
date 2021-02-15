use crate::StructuralSignal;
use im::hashmap;
use im::HashMap; // Doesn't need to be immutable, but no need to pull in another HashMap.
use parking_lot::RwLock;
use pin_project::pin_project;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub(crate) type DiffNumber = usize;
pub(crate) type SignalId = usize;

pub enum DiffMergeOutcome<DiffType: PullableDiff> {
    // Merge the new diff and the previous diff together into a new DiffType instance.
    Merge(DiffType),

    // Keep only the new diff, discarding the previous one.
    Replace,

    // Keep only the previous diff, discarding the new one. This is useful for situations
    // like Insert then Update where the Insert remains the canonically correct diff.
    Ignore,

    // Keep both the new diff and the old diff. Note that this solidifies the previous
    // diff, making it impossible to optimize out later. Avoid this where possible. It
    // is necessary for things like array deletion that can overlap on the same index.
    KeepBoth,

    // The diffs cancel out, discard both. Big win for efficiency!
    // Items in between the two diffs may now have the wrong index, so the optional
    // reindexer function will run on affected diffs.
    DiscardBoth,
}

pub struct DiffMergeResult<DiffType: PullableDiff> {
    outcome: DiffMergeOutcome<DiffType>,
    reindex_intermediary_diffs: Option<Box<dyn Fn(&DiffType::KeyType, &DiffType::KeyType) -> DiffType::KeyType>>,
}

impl<DiffType: PullableDiff> DiffMergeResult<DiffType> {
    pub fn merge(diff: DiffType) -> Self {
        DiffMergeResult {
            outcome: DiffMergeOutcome::Merge(diff),
            reindex_intermediary_diffs: None,
        }
    }

    pub fn merge_and_reindex<F>(diff: DiffType, reindex: F) -> Self
    where
        F: Fn(&DiffType::KeyType, &DiffType::KeyType) -> DiffType::KeyType,
        F: 'static,
    {
        DiffMergeResult {
            outcome: DiffMergeOutcome::Merge(diff),
            reindex_intermediary_diffs: Some(Box::new(reindex)),
        }
    }

    pub fn replace() -> Self {
        DiffMergeResult {
            outcome: DiffMergeOutcome::Replace,
            reindex_intermediary_diffs: None,
        }
    }

    pub fn replace_and_reindex<F>(reindex: F) -> Self
    where
        F: Fn(&DiffType::KeyType, &DiffType::KeyType) -> DiffType::KeyType,
        F: 'static,
    {
        DiffMergeResult {
            outcome: DiffMergeOutcome::Replace,
            reindex_intermediary_diffs: Some(Box::new(reindex)),
        }
    }

    pub fn ignore() -> Self {
        DiffMergeResult {
            outcome: DiffMergeOutcome::Ignore,
            reindex_intermediary_diffs: None,
        }
    }

    pub fn keep_both() -> Self {
        DiffMergeResult {
            outcome: DiffMergeOutcome::KeepBoth,
            reindex_intermediary_diffs: None,
        }
    }

    pub fn discard_both() -> Self {
        DiffMergeResult {
            outcome: DiffMergeOutcome::DiscardBoth,
            reindex_intermediary_diffs: None,
        }
    }

    pub fn discard_both_and_reindex<F>(reindex: F) -> Self
    where
        F: Fn(&DiffType::KeyType, &DiffType::KeyType) -> DiffType::KeyType,
        F: 'static,
    {
        DiffMergeResult {
            outcome: DiffMergeOutcome::DiscardBoth,
            reindex_intermediary_diffs: Some(Box::new(reindex)),
        }
    }
}

pub trait PullableDiff: Clone
where
    Self: Sized,
{
    type KeyType: Clone + Eq + Hash;

    fn get_key(&self) -> Option<&Self::KeyType>;
    fn get_snapshot_key(&self) -> Option<&Self::KeyType>;
    fn set_key(&mut self, new_key: Self::KeyType);
    fn set_snapshot_key(&mut self, new_snapshot_key: Self::KeyType);
    fn merge_with_previous(&self, previous: &Self) -> DiffMergeResult<Self>;
    fn full_replace() -> Self;
}

pub trait PullSourceHost
where
    Self: Sized,
{
    type DiffType: PullableDiff;
    type EventType: Clone;

    fn get_pull_source<'a>(&'a mut self) -> &'a mut StructrualSignalPullSource<Self::DiffType>;
    fn make_event(&self, diffs: Vec<Self::DiffType>) -> Self::EventType;
}

/// A PullSource is a more efficient way of broadcasting StructrualSignals than channel broadcasting
/// because it can batch diffs together into a smaller number of events. It can also operate lazily,
/// not tracking any changes until the first signal starts polling for changes.
#[derive(Debug)]
pub struct StructrualSignalPullSource<DiffType: PullableDiff> {
    diffs: BTreeMap<DiffNumber, DiffType>,
    signal_last_diff_numbers: BTreeMap<SignalId, DiffNumber>,
    diffs_per_key: HashMap<DiffType::KeyType, DiffNumber>,
    next_diff_index: DiffNumber,
    next_signal_id: SignalId,
}

impl<DiffType: PullableDiff> StructrualSignalPullSource<DiffType> {
    pub(crate) fn new() -> StructrualSignalPullSource<DiffType> {
        StructrualSignalPullSource {
            diffs: BTreeMap::new(),
            signal_last_diff_numbers: BTreeMap::new(),
            diffs_per_key: HashMap::new(),
            next_diff_index: 1,
            next_signal_id: 1,
        }
    }
}

impl<DiffType: PullableDiff> StructrualSignalPullSource<DiffType> {
    pub fn add_diff(&mut self, mut diff: DiffType) {
        if !self.has_listening_signal() {
            return;
        }

        let maybe_diff_key = diff.get_key().map(|key| key.clone());
        if let Some(diff_key) = maybe_diff_key {
            if self.diffs_per_key.contains_key(&diff_key) {
                let prev_diff_index = *self.diffs_per_key.get(&diff_key).unwrap();
                let prev_diff = self.diffs.remove(&prev_diff_index).unwrap();
                let was_diff_for_key = self.diffs_per_key.remove(&diff_key).is_some();
                let result = diff.merge_with_previous(&prev_diff);

                if let Some(reindexer) = result.reindex_intermediary_diffs {
                    self.update_intermediary_diffs(prev_diff_index, reindexer);
                }

                match result.outcome {
                    DiffMergeOutcome::Merge(merged) => {
                        diff = merged;
                    }
                    DiffMergeOutcome::Replace => {
                        // Keep the new diff, and ensure the old one stays discarded. NoOp.
                    }
                    DiffMergeOutcome::Ignore => {
                        // Undo the changes that have been done so far and halt.
                        self.diffs.insert(prev_diff_index, prev_diff);
                        if was_diff_for_key {
                            self.diffs_per_key.insert(diff_key, prev_diff_index);
                        }
                        return;
                    }
                    DiffMergeOutcome::KeepBoth => {
                        // The previous value will be replaced in diffs_per_key, but should still
                        // exist in the diffs list.
                        self.diffs.insert(prev_diff_index, prev_diff);
                    }
                    DiffMergeOutcome::DiscardBoth => {
                        // If the two diffs cancel out, the first one is already removed, and
                        // the second one should simply not be added.
                        return;
                    }
                }
            }

            // This diff will stand in for the current key.
            self.diffs_per_key
                .insert(diff_key.clone(), self.next_diff_index);
        } else {
            // If this diff affects no particular key, assume it affects _all_ keys.
            // Think: Replace or Clear.
            // In these cases, all previous diffs are now irrelevant and can be wiped out.
            self.diffs.clear();
            self.diffs_per_key.clear();
        }
        self.diffs.insert(self.next_diff_index, diff);
        self.next_diff_index += 1;
    }

    pub fn pull_signal(&mut self, signal_id: SignalId) -> Vec<DiffType> {
        let current_diff_number = self.next_diff_index - 1;
        let maybe_last_diff_number = self
            .signal_last_diff_numbers
            .insert(signal_id, current_diff_number);

        if maybe_last_diff_number.is_some()
            && maybe_last_diff_number.unwrap() == current_diff_number
        {
            return vec![];
        }

        // As soon as a diff is returned, it can no longer be optimized out
        // of existence, so the map responsible for that is cleared.
        self.diffs_per_key.clear();

        if let None = maybe_last_diff_number {
            return vec![DiffType::full_replace()];
        };

        let start_at = maybe_last_diff_number.unwrap() + 1;
        let diffs_in_range: Vec<DiffType> = self
            .diffs
            .range(start_at..)
            .map(|(_k, v)| v.clone())
            .collect();

        // If any of the diffs affect every key (think Replace or Clear) then the
        // end result of this diff will be an entirely different data set from the
        // original, therefore a full replace is appropriate.
        let has_global_diff = diffs_in_range
            .iter()
            .find(|diff| diff.get_key().is_none())
            .is_some();
        if has_global_diff {
            return vec![DiffType::full_replace()];
        }

        return diffs_in_range;
    }

    pub fn update_keys<F>(&mut self, updater: F)
    where
        F: Fn(&DiffType::KeyType) -> DiffType::KeyType,
    {
        let mut tmp_diffs_per_key = hashmap! {};
        std::mem::swap(&mut self.diffs_per_key, &mut tmp_diffs_per_key);
        self.diffs_per_key = tmp_diffs_per_key
            .into_iter()
            .map(|(k, v)| (updater(&k), v))
            .collect();

        for (_index, diff) in self.diffs.iter_mut() {
            if let Some(existing_key) = diff.get_snapshot_key() {
                let updated = updater(existing_key);
                diff.set_snapshot_key(updated);
            }
        }
    }

    // When two diffs overlap on a key, sometimes they are combined into one diff.
    // Any diffs between those two may need to have their index updated.
    fn update_intermediary_diffs<F>(&mut self, start_diff: usize, updater: F)
    where
        F: Fn(&DiffType::KeyType, &DiffType::KeyType) -> DiffType::KeyType,
    {
        for i in start_diff..self.next_diff_index {
            if let Some(diff) = self.diffs.get_mut(&i) {
                if let Some(key) = diff.get_key() {
                    if let Some(snapshot_key) = diff.get_snapshot_key() {
                        diff.set_key(updater(key, snapshot_key));
                    } else {
                        panic!("Diff had a key but not a snapshot key");
                    }
                }
            }
        }
    }

    pub fn get_next_signal_id(&mut self) -> SignalId {
        let next_id = self.next_signal_id;
        self.next_signal_id += 1;
        return next_id;
    }

    pub fn has_listening_signal(&self) -> bool {
        self.signal_last_diff_numbers.len() > 0
    }
}

/// A Signal derived from a PullSource.
#[pin_project(project = PullSourceStructuralSignalProj)]
pub struct PullSourceStructuralSignal<H>
where
    H: PullSourceHost,
{
    id: SignalId,
    pull_source_host: Arc<RwLock<H>>,
}

impl<H> PullSourceStructuralSignal<H>
where
    H: PullSourceHost,
{
    pub(crate) fn new(pull_source_host: Arc<RwLock<H>>) -> PullSourceStructuralSignal<H> {
        let id = pull_source_host
            .write()
            .get_pull_source()
            .get_next_signal_id();
        PullSourceStructuralSignal {
            id,
            pull_source_host,
        }
    }
}

impl<H> StructuralSignal for PullSourceStructuralSignal<H>
where
    H: PullSourceHost,
{
    type Item = H::EventType;

    fn poll_change(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<H::EventType>> {
        let diffs = self
            .pull_source_host
            .write()
            .get_pull_source()
            .pull_signal(self.id);
        if diffs.is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(Some(self.pull_source_host.read().make_event(diffs)))
        }
    }
}
