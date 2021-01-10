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

pub trait PullableDiff: Clone
where
    Self: Sized,
{
    type KeyType: Clone + Eq + Hash;

    fn get_key(&self) -> Option<&Self::KeyType>;
    fn set_key(&mut self, new_key: Self::KeyType);
    fn merge_with_previous(self, previous: Self) -> Option<Self>;
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
                let prev_diff = self
                    .diffs
                    .remove(self.diffs_per_key.get(&diff_key).unwrap())
                    .unwrap();
                self.diffs_per_key.remove(&diff_key);
                let maybe_merged = diff.merge_with_previous(prev_diff);
                if let Some(merged) = maybe_merged {
                    diff = merged;
                } else {
                    // If the two diffs cancel out, the first one is already removed, and
                    // the second one should simply not be added.
                    return;
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
        let maybe_last_diff_number = self.signal_last_diff_numbers.get(&signal_id);
        let results = if let Some(last_diff_number) = maybe_last_diff_number {
            if *last_diff_number == current_diff_number {
                return vec![];
            }

            self.diffs
                .range(last_diff_number..)
                .map(|(_k, v)| v.clone())
                .collect()
        } else {
            vec![DiffType::full_replace()]
        };
        self.signal_last_diff_numbers
            .insert(signal_id, current_diff_number);
        return results;
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

        for (new_key, diff_index) in self.diffs_per_key.iter() {
            self.diffs
                .get_mut(diff_index)
                .unwrap()
                .set_key(new_key.clone());
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
