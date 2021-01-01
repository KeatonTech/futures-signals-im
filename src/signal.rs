use std::pin::Pin;
use std::task::{Context, Poll};

#[must_use = "SignalMaps do nothing unless polled"]
pub trait StructuralSignal {
    type Item: Clone;

    fn poll_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>>;
}

// Copied from Future in the Rust stdlib
impl<'a, A> StructuralSignal for &'a mut A
where
    A: ?Sized + StructuralSignal + Unpin,
{
    type Item = A::Item;

    #[inline]
    fn poll_change(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
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
    fn poll_change(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
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
    fn poll_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        Pin::get_mut(self).as_mut().poll_change(cx)
    }
}
