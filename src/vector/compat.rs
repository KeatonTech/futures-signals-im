use super::event::{VectorDiff, VectorEvent, VectorIndex};
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
}

impl<S, T> From<S> for StructuralSignalVecCompat<S, T>
where
    T: Clone,
    S: StructuralSignal<Item = VectorEvent<T>>,
    S: Unpin,
{
    fn from(inner: S) -> StructuralSignalVecCompat<S, T> {
        StructuralSignalVecCompat { inner }
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
        let StructuralSignalVecCompatProj { inner } = self.project();
        inner.poll_change(cx).map(|maybe_vector_event| {
            maybe_vector_event.map(|vector_event| match vector_event.diff {
                VectorDiff::Replace {} => VecDiff::Replace {
                    values: vector_event.snapshot.clone().into_iter().collect(),
                },
                VectorDiff::Insert { index } => match index {
                    VectorIndex::Index(idx) => VecDiff::InsertAt {
                        index: idx,
                        value: vector_event.snapshot[idx].clone(),
                    },
                    VectorIndex::LastIndex => VecDiff::Push {
                        value: vector_event.snapshot.back().unwrap().clone(),
                    },
                },
                VectorDiff::Update { index } => match index {
                    VectorIndex::Index(idx) => VecDiff::UpdateAt {
                        index: idx,
                        value: vector_event.snapshot[idx].clone(),
                    },
                    VectorIndex::LastIndex => VecDiff::UpdateAt {
                        index: vector_event.snapshot.len() - 1,
                        value: vector_event.snapshot.back().unwrap().clone(),
                    },
                },
                VectorDiff::Remove { index } => match index {
                    VectorIndex::Index(idx) => VecDiff::RemoveAt { index: idx },
                    VectorIndex::LastIndex => VecDiff::Pop {},
                },
                VectorDiff::Clear {} => VecDiff::Clear {},
            })
        })
    }
}
