//! Stack implementation

#[cfg(feature = "arc")]
/// `Arc`-based stack
mod arc_stack;
#[cfg(feature = "rc")]
/// `Rc`-based stack
mod rc_stack;

#[cfg(feature = "arc")]
#[allow(deprecated)]
pub use arc_stack::{ArcCsStack, ArcLocalStack, ArcStack};
#[cfg(feature = "rc")]
pub use rc_stack::RcStack;
