//! Shared abstractions used across API and internal layers.

mod actor;
pub mod mailbox;
pub mod messaging;
pub mod supervision;

pub use actor::TypedHandlerBridge;
