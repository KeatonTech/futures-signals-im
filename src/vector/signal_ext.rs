use super::{VectorEvent};
use super::vector_transforms::{MapVectorTransformer};
use crate::structural_signal::transformer::TransformedStructuralSignal;
use crate::StructuralSignal;

pub trait SignalVectorExt: StructuralSignal
where
    Self: Sized,
{
    type ValType: Clone;
    type SelfType: StructuralSignal<Item = VectorEvent<Self::ValType>>;

    /// Returns a version of this signal where every value in the map has been run
    /// through a transformer function.
    ///
    /// ```
    /// use signals_im::vector::{MutableVector, SignalVectorExt};
    /// use signals_im::StructuralSignalExt;
    /// use im::vector;
    ///
    /// let input_vec = MutableVector::<u8>::new();
    /// input_vec.write().push_back(1);
    /// input_vec.write().push_back(2);
    ///
    /// let multiplied = input_vec.as_signal().map(|v| v * 2);
    /// input_vec.write().push_front(0);
    ///
    /// let multiplied_vec = multiplied.snapshot().unwrap();
    /// assert_eq!(multiplied_vec, vector![0, 2, 4]);
    /// ```
    fn map<OV, F>(
        self,
        map_fn: F,
    ) -> TransformedStructuralSignal<
        Self::SelfType,
        <Self::SelfType as StructuralSignal>::Item,
        MapVectorTransformer<F, Self::ValType, OV>,
    >
    where
        OV: Clone,
        Self::ValType: Clone,
        F: Fn(&Self::ValType) -> OV;
}

impl<T, I> SignalVectorExt for I
where
    I: StructuralSignal<Item = VectorEvent<T>>,
    T: Clone,
{
    type ValType = T;
    type SelfType = I;

    fn map<OV, F>(
        self,
        map_fn: F,
    ) -> TransformedStructuralSignal<
        Self,
        Self::Item,
        MapVectorTransformer<F, Self::ValType, OV>,
    >
    where
        OV: Clone,
        Self::ValType: Clone,
        F: Fn(&Self::ValType) -> OV,
        Self: Sized,
    {
        TransformedStructuralSignal::new(self, MapVectorTransformer::new(map_fn))
    }
}