use crate::StructuralSignal;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A trait that takes a series of events from an input StructuralSignal and
/// applies it to an internal data structure to create a modified output
/// signal. This is used for operations like mapping and filtering structures.
pub trait StructuralSignalTransformer {
    type InputEvent;
    type OutputSignal: StructuralSignal;

    fn apply_event(&mut self, event: Self::InputEvent);
    fn get_signal(&self) -> Self::OutputSignal;
}

/// A StructuralSignal that has been run through a StructuralSignalTransformer.
#[pin_project(project = TransformedStructuralSignalProj)]
pub struct TransformedStructuralSignal<IS, II, T>
where
    IS: StructuralSignal<Item=II>,
    II: Clone,
    T: StructuralSignalTransformer<InputEvent=II>,
{
    #[pin]
    input_signal: IS,
    #[pin]
    transformed_signal: T::OutputSignal,
    transformer: T,

    is_closed: bool,
}

impl<IS, II, T> TransformedStructuralSignal<IS, II, T>
where
    IS: StructuralSignal<Item=II>,
    II: Clone,
    T: StructuralSignalTransformer<InputEvent=II>,
{
    pub(crate) fn new(input_signal: IS, transformer: T) -> TransformedStructuralSignal<IS, II, T> {
        let transformed_signal = transformer.get_signal();
        TransformedStructuralSignal {
            input_signal,
            transformed_signal,
            transformer,
            is_closed: false,
        }
    }
}

impl<IS, II, T> StructuralSignal for TransformedStructuralSignal<IS, II, T>
where
    IS: StructuralSignal<Item=II>,
    II: Clone,
    T: StructuralSignalTransformer<InputEvent=II>,
{
    type Item = <T::OutputSignal as StructuralSignal>::Item;

    #[inline]
    fn poll_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let TransformedStructuralSignalProj {
            mut input_signal,
            transformed_signal,
            transformer,
            is_closed,
        } = self.project();

        loop {
            let input_poll = input_signal.as_mut().poll_change(cx);
            match input_poll {
                Poll::Ready(Some(event)) => {
                    transformer.apply_event(event);
                }
                Poll::Ready(None) => {
                    *is_closed = true;
                    break;
                }
                Poll::Pending => {
                    break;
                }
            }
        }

        let result = transformed_signal.poll_change(cx);
        if *is_closed && !result.is_ready() {
            Poll::Ready(None)
        } else {
            result
        }
    }
}