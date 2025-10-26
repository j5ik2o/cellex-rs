//! v2 collection adaptors for std environments.

pub mod async_queue;
mod sync_queue;
mod sync_stack;

pub use sync_queue::*;
pub use sync_stack::*;
