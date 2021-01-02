use crate::StructuralSignal;
use futures::channel::mpsc;
use futures_util::stream::StreamExt;
use parking_lot::RwLock;
use pin_project::pin_project;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub trait StructuralSignalExt: StructuralSignal
where
    Self: Sized,
{
    /// Converts this StructuralSignal a StructuralSignalBroadcaster, which can
    /// distribute events to multiple signals. This allows signals to be effectively
    /// cloned, while ensuring upstream signal transformers only have to run once.
    ///
    /// ```
    /// use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
    /// use signals_im::StructuralSignalExt;
    /// use im::hashmap;
    ///
    /// let input_map = MutableHashMap::<u8, u8>::new();
    /// input_map.write().insert(1, 1);
    ///
    /// // This transform will only occur once for each map update, regardless of
    /// // how many signals the broadcaster creates.
    /// let broadcaster = input_map.as_signal().map_values(|v| v * 2).broadcast();
    /// assert_eq!(broadcaster.get_signal().into_map_sync().unwrap(), hashmap!{1 => 2});
    ///
    /// input_map.write().insert(2, 2);
    /// assert_eq!(broadcaster.get_signal().into_map_sync().unwrap(), hashmap!{1 => 2, 2 => 4});
    /// ```
    fn broadcast(self) -> StructuralSignalBroadcaster<Self::Item, Self>
    where
        Self: Unpin,
        Self::Item: Clone;
}

impl<I> StructuralSignalExt for I
where
    I: StructuralSignal,
    I: Sized,
{
    fn broadcast(self) -> StructuralSignalBroadcaster<Self::Item, Self>
    where
        Self: Unpin,
        Self::Item: Clone,
    {
        StructuralSignalBroadcaster::new(self)
    }
}

#[pin_project(project = StructuralSignalBroadcasterStateProj)]
pub struct StructuralSignalBroadcasterState<I, S>
where
    I: Clone,
    S: StructuralSignal<Item = I>,
    S: Unpin,
{
    #[pin]
    input: S,
    most_recent_event: Option<I>,
    senders: Vec<Option<mpsc::UnboundedSender<I>>>,
}

impl<I, S> StructuralSignalBroadcasterState<I, S>
where
    I: Clone,
    S: StructuralSignal<Item = I>,
    S: Unpin,
{
    fn pull_in_new_changes(self: Pin<&mut Self>, cx: &mut Context) -> bool {
        let StructuralSignalBroadcasterStateProj {
            input,
            most_recent_event,
            senders,
        } = self.project();
        let poll_channel = input.poll_change(cx);
        if let Poll::Ready(maybe_event) = &poll_channel {
            if let Some(event) = maybe_event {
                most_recent_event.replace(event.clone());
            }

            for maybe_sender in senders.iter_mut() {
                if let Some(sender) = maybe_sender {
                    if sender.is_closed() {
                        maybe_sender.take();
                    } else if (&maybe_event).is_some() {
                        sender.unbounded_send(maybe_event.clone().unwrap()).unwrap();
                    } else {
                        sender.close_channel();
                    }
                }
            }
            return true;
        } else {
            return false;
        }
    }
}

pub struct StructuralSignalBroadcaster<I, S>(Arc<RwLock<StructuralSignalBroadcasterState<I, S>>>)
where
    I: Clone,
    S: StructuralSignal<Item = I>,
    S: Unpin;

impl<I, S> StructuralSignalBroadcaster<I, S>
where
    I: Clone,
    S: StructuralSignal<Item = I>,
    S: Unpin,
{
    pub(crate) fn new(input: S) -> StructuralSignalBroadcaster<I, S> {
        StructuralSignalBroadcaster(Arc::new(RwLock::new(StructuralSignalBroadcasterState {
            input: input,
            most_recent_event: None,
            senders: vec![],
        })))
    }

    pub fn get_signal(&self) -> BroadcastedStructuralSignal<I, S> {
        let (sender, receiver) = mpsc::unbounded();

        {
            let most_recent_event = &self.0.read().most_recent_event;
            if let Some(event) = most_recent_event {
                sender.unbounded_send(event.clone()).unwrap();
            }
        }

        self.0.write().senders.push(Some(sender));
        BroadcastedStructuralSignal {
            receiver: receiver,
            parent: self.0.clone(),
        }
    }
}

#[pin_project(project = BroadcastedStructuralSignalProj)]
pub struct BroadcastedStructuralSignal<I, S>
where
    I: Clone,
    S: StructuralSignal<Item = I>,
    S: Unpin,
{
    receiver: mpsc::UnboundedReceiver<I>,
    parent: Arc<RwLock<StructuralSignalBroadcasterState<I, S>>>,
}

impl<I, S> StructuralSignal for BroadcastedStructuralSignal<I, S>
where
    I: Clone,
    S: StructuralSignal<Item = I>,
    S: Unpin,
{
    type Item = I;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<I>> {
        let BroadcastedStructuralSignalProj { receiver, parent } = self.project();
        let poll_channel = receiver.poll_next_unpin(cx);
        if let Poll::Ready(result) = poll_channel {
            return Poll::Ready(result);
        }

        let mut writer = parent.write();
        let has_new_changes = Pin::new(&mut *writer).pull_in_new_changes(cx);
        if has_new_changes {
            receiver.poll_next_unpin(cx)
        } else {
            Poll::Pending
        }
    }
}
