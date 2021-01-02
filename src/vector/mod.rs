pub mod compat;
mod event;
mod vector;

pub use event::{VectorDiff, VectorEvent};
pub use vector::{MutableVector, MutableVectorReader};