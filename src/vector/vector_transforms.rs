use super::{MutableVector, MutableVectorState, VectorDiff, VectorEvent};
use crate::structural_signal::pull_source::PullSourceStructuralSignal;
use crate::structural_signal::transformer::StructuralSignalTransformer;
use std::marker::PhantomData;

// ** MAP ** //

pub struct MapVectorTransformer<F, IV, OV>
where
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    vector: MutableVector<OV>,
    map_fn: F,
    input_type: PhantomData<IV>,
}

impl<F, IV, OV> MapVectorTransformer<F, IV, OV>
where
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    pub(crate) fn new(map_fn: F) -> MapVectorTransformer<F, IV, OV> {
        MapVectorTransformer {
            vector: MutableVector::new(),
            map_fn: map_fn,
            input_type: PhantomData,
        }
    }
}

impl<F, IV, OV> StructuralSignalTransformer for MapVectorTransformer<F, IV, OV>
where
    IV: Clone,
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    type InputEvent = VectorEvent<IV>;
    type OutputSignal = PullSourceStructuralSignal<MutableVectorState<OV>>;

    fn apply_event(&mut self, map_event: VectorEvent<IV>) {
        let mut writer = self.vector.write();
        for diff in map_event.diffs {
            match diff {
                VectorDiff::Replace {} => {
                    writer.replace(
                        map_event
                            .snapshot
                            .clone()
                            .into_iter()
                            .map(|v| (self.map_fn)(&v)),
                    );
                }
                VectorDiff::Insert {
                    index,
                    snapshot_index: _,
                } => {
                    let mapped_val =
                        (self.map_fn)(&diff.get_value_from_snapshot(&map_event.snapshot).unwrap());
                    writer.insert(index, mapped_val);
                }
                VectorDiff::Update {
                    index,
                    snapshot_index: _,
                } => {
                    let mapped_val =
                        (self.map_fn)(&diff.get_value_from_snapshot(&map_event.snapshot).unwrap());
                    writer.set(index, mapped_val);
                }
                VectorDiff::Remove {
                    index,
                    snapshot_index: _,
                } => {
                    writer.remove(index);
                }
                VectorDiff::Clear {} => {
                    writer.clear();
                }
            }
        }
    }

    #[inline]
    fn get_signal(&self) -> Self::OutputSignal {
        self.vector.as_signal()
    }
}
