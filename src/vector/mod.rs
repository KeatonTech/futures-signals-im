pub mod compat;
mod event;
mod vector;
mod vector_transforms;
mod signal_ext;

pub use event::{VectorDiff, VectorEvent};
pub use vector::{MutableVector, MutableVectorReader, MutableVectorState};
pub use signal_ext::SignalVectorExt;