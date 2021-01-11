use crate::structural_signal::pull_source::PullSourceStructuralSignal;
use crate::structural_signal::transformer::StructuralSignalTransformer;
use super::{MutableVector, MutableVectorState, VectorEvent, VectorDiff};
use std::marker::PhantomData;

// ** MAP ** //

pub struct MapVectorTransformer<F, IV, OV>
where
    OV: Clone,
    F: Fn(&IV) -> OV,
{
    hash_map: MutableVector<OV>,
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
            hash_map: MutableVector::new(),
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
        let mut writer = self.hash_map.write();
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
                VectorDiff::Insert { index } | VectorDiff::Update { index } => {
                    let mapped_val = (self.map_fn)(map_event.snapshot.get(index).unwrap());
                    writer.insert(index, mapped_val);
                }
                VectorDiff::Remove { index } => {
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
        self.hash_map.as_signal()
    }
}