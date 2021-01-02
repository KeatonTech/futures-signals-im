pub mod hash_map;
pub mod vector;
mod structural_signal;
mod structural_signal_ext;
mod transformer;
pub(crate) mod util;

pub use structural_signal::{StructuralSignal, ChannelStructuralSignal};
pub use structural_signal_ext::StructuralSignalExt;