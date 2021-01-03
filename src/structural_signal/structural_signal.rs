use futures::channel::mpsc;
use futures_util::stream::StreamExt;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Similar to futures_signals::Signal, but does not skip any values such
/// that data structure mutations cannot be missed.
#[must_use = "SignalMaps do nothing unless polled"]
pub trait StructuralSignal {
    type Item: Clone;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;
}

// Copied from Future in the Rust stdlib
impl<'a, A> StructuralSignal for &'a mut A
where
    A: ?Sized + StructuralSignal + Unpin,
{
    type Item = A::Item;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        A::poll_change(Pin::new(&mut **self), cx)
    }
}

// Copied from Future in the Rust stdlib
impl<A> StructuralSignal for Box<A>
where
    A: ?Sized + StructuralSignal + Unpin,
{
    type Item = A::Item;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        A::poll_change(Pin::new(&mut *self), cx)
    }
}

// Copied from Future in the Rust stdlib
impl<A> StructuralSignal for Pin<A>
where
    A: Unpin + ::std::ops::DerefMut,
    A::Target: StructuralSignal,
{
    type Item = <<A as ::std::ops::Deref>::Target as StructuralSignal>::Item;

    #[inline]
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::get_mut(self).as_mut().poll_change(cx)
    }
}

/// A basic implementation of StructuralSignal backed by an mpsc unbounded receiver.
#[doc(hidden)]
pub struct ChannelStructuralSignal<I: Clone> {
    receiver: mpsc::UnboundedReceiver<I>,
}

impl<I: Clone> ChannelStructuralSignal<I> {
    pub fn new(receiver: mpsc::UnboundedReceiver<I>) -> ChannelStructuralSignal<I> {
        ChannelStructuralSignal { receiver }
    }
}

impl<I: Clone> Unpin for ChannelStructuralSignal<I> {}

impl<I: Clone> StructuralSignal for ChannelStructuralSignal<I> {
    type Item = I;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<I>> {
        self.receiver.poll_next_unpin(cx)
    }
}
