use super::event::HashMapEvent;
use core::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};

#[must_use = "SignalMaps do nothing unless polled"]
pub trait SignalHashMap {
    type Key: Clone + Eq + Hash;
    type Value: Clone;

    fn poll_map_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<HashMapEvent<Self::Key, Self::Value>>>;
}

// Copied from Future in the Rust stdlib
impl<'a, A> SignalHashMap for &'a mut A
where
    A: ?Sized + SignalHashMap + Unpin,
{
    type Key = A::Key;
    type Value = A::Value;

    #[inline]
    fn poll_map_change(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<HashMapEvent<Self::Key, Self::Value>>> {
        A::poll_map_change(Pin::new(&mut **self), cx)
    }
}

// Copied from Future in the Rust stdlib
impl<A> SignalHashMap for Box<A>
where
    A: ?Sized + SignalHashMap + Unpin,
{
    type Key = A::Key;
    type Value = A::Value;

    #[inline]
    fn poll_map_change(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<HashMapEvent<Self::Key, Self::Value>>> {
        A::poll_map_change(Pin::new(&mut *self), cx)
    }
}

// Copied from Future in the Rust stdlib
impl<A> SignalHashMap for Pin<A>
where
    A: Unpin + ::std::ops::DerefMut,
    A::Target: SignalHashMap,
{
    type Key = <<A as ::std::ops::Deref>::Target as SignalHashMap>::Key;
    type Value = <<A as ::std::ops::Deref>::Target as SignalHashMap>::Value;

    #[inline]
    fn poll_map_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<HashMapEvent<Self::Key, Self::Value>>> {
        Pin::get_mut(self).as_mut().poll_map_change(cx)
    }
}
