pub mod hash_map;
pub mod vector;
pub(crate) mod structural_signal;
pub(crate) mod util;

pub use structural_signal::structural_signal::{StructuralSignal, ChannelStructuralSignal};
pub use structural_signal::structural_signal_ext::{StructuralSignalExt, SnapshottableEvent};