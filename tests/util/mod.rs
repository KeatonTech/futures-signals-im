use futures_executor::block_on;
use futures_util::future::poll_fn;
use im::hashmap::HashMap;
use pin_utils::pin_mut;
use signals_im::hash_map::{HashMapEvent, MapDiff};
use signals_im::StructuralSignal;
use std::hash::Hash;
use std::task::Poll;

/// Result of a full poll on a StructuralSignal
pub struct PollResult<I> {
    pub items: Vec<I>,
    pub is_done: bool,
}

/// Performs a poll on a StructuralSignal, returning every item that was output
/// and indicating whether the signal is 'done'.
pub fn poll_all<S>(signal: &mut S) -> PollResult<S::Item>
where
    S: StructuralSignal,
    S: Unpin,
{
    pin_mut!(signal);

    let mut items = vec![];
    let mut is_done = false;

    block_on(poll_fn(|cx| loop {
        match signal.as_mut().poll_change(cx) {
            Poll::Ready(Some(val)) => {
                items.push(val);
                continue;
            }
            Poll::Ready(None) => {
                is_done = true;
                break Poll::Ready(None as Option<bool>);
            }
            Poll::Pending => {
                break Poll::Ready(None as Option<bool>);
            }
        };
    }));

    PollResult { items, is_done }
}

/// Extracts a list of snapshots from a list of HashMapEvents.
pub fn get_snapshots<K, V>(events: &Vec<HashMapEvent<K, V>>) -> Vec<HashMap<K, V>>
where
    K: Clone + Hash + Eq,
    V: Clone,
{
    events
        .clone()
        .into_iter()
        .map(|event| event.snapshot)
        .collect()
}

/// Extracts a list of diffs from a list of HashMapEvents.
pub fn get_hash_map_diffs<K, V>(events: &Vec<HashMapEvent<K, V>>) -> Vec<MapDiff<K>>
where
    K: Clone + Hash + Eq,
    V: Clone,
{
    events
        .clone()
        .into_iter()
        .map(|event| event.diffs)
        .flatten()
        .collect()
}
