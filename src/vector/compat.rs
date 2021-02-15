use super::event::{VectorDiff, VectorEvent};
use crate::StructuralSignal;
use futures_signals::signal_vec::{SignalVec, VecDiff};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

#[pin_project(project = StructuralSignalVecCompatProj)]
struct StructuralSignalVecCompat<S, T>
where
    T: Clone,
    S: StructuralSignal<Item = VectorEvent<T>>,
    S: Unpin,
{
    #[pin]
    inner: S,
    // Using this instead of VecDeque to save space.
    last_event: Option<VectorEvent<T>>,
    diffs_buffer_next_index: usize,
    length: usize,
}

impl<S, T> From<S> for StructuralSignalVecCompat<S, T>
where
    T: Clone,
    S: StructuralSignal<Item = VectorEvent<T>>,
    S: Unpin,
{
    fn from(inner: S) -> StructuralSignalVecCompat<S, T> {
        StructuralSignalVecCompat {
            inner,
            last_event: None,
            diffs_buffer_next_index: 0,
            length: 0,
        }
    }
}

impl<S, T> StructuralSignalVecCompat<S, T>
where
    T: Clone,
    S: StructuralSignal<Item = VectorEvent<T>>,
    S: Unpin,
{
    fn convert_next_diff(
        last_event: &mut VectorEvent<T>,
        diffs_buffer_next_index: &mut usize,
        length: &mut usize,
    ) -> Option<VecDiff<T>> {
        let snapshot = &last_event.snapshot;
        let vector_diff = &last_event.diffs[*diffs_buffer_next_index];

        Some(match vector_diff {
            VectorDiff::Replace {} => {
                *length = snapshot.len();
                VecDiff::Replace {
                    values: snapshot.clone().into_iter().collect(),
                }
            }
            VectorDiff::Insert {
                index, 
                snapshot_index: _,
            } => {
                *length += 1;
                if *index == *length - 1 {
                    VecDiff::Push {
                        value: snapshot.back().unwrap().clone(),
                    }
                } else {
                    VecDiff::InsertAt {
                        index: *index,
                        value: snapshot[*index].clone(),
                    }
                }
            },
            VectorDiff::Update {
                index,
                snapshot_index: _,
            } => VecDiff::UpdateAt {
                index: *index,
                value: snapshot[*index].clone(),
            },
            VectorDiff::Remove {
                index,
                snapshot_index: _,
            } => {
                *length -= 1;
                if *index == *length {
                    VecDiff::Pop {}
                } else {
                    VecDiff::RemoveAt { index: *index }
                }
            }
            VectorDiff::Clear {} => VecDiff::Clear {},
        })
    }
}

impl<S, T> SignalVec for StructuralSignalVecCompat<S, T>
where
    T: Clone,
    S: StructuralSignal<Item = VectorEvent<T>>,
    S: Unpin,
{
    type Item = T;

    #[inline]
    fn poll_vec_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<VecDiff<Self::Item>>> {
        let StructuralSignalVecCompatProj {
            inner,
            last_event,
            diffs_buffer_next_index,
            length,
        } = self.project();

        if last_event.is_none() {
            match inner.poll_change(cx) {
                Poll::Ready(Some(event)) => {
                    *last_event = Some(event);
                    *diffs_buffer_next_index = 0;
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        let result = StructuralSignalVecCompat::<S, T>::convert_next_diff(
            last_event.as_mut().unwrap(),
            diffs_buffer_next_index,
            length,
        );

        if *diffs_buffer_next_index == last_event.as_ref().unwrap().diffs.len() - 1 {
            *last_event = None;
            *diffs_buffer_next_index = 0;
        }

        return Poll::Ready(result);
    }
}
