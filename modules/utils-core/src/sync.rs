#[allow(clippy::disallowed_types)]
mod arc_shared;
/// Async-aware mutex abstractions shared across runtimes.
pub mod async_mutex_like;
#[allow(clippy::disallowed_types)]
mod flag;
/// Helper traits for shared function and factory closures.
pub mod function;
/// Policies for detecting interrupt contexts prior to blocking operations.
pub mod interrupt;
#[cfg(feature = "alloc")]
#[allow(clippy::disallowed_types)]
mod rc_shared;
/// Shared ownership utilities.
pub mod shared;
mod state;
mod static_ref_shared;
/// Synchronous mutex abstractions shared across runtimes.
pub mod sync_mutex_like;

pub use arc_shared::ArcShared;
pub use flag::Flag;
#[cfg(feature = "alloc")]
pub use rc_shared::RcShared;
pub use state::StateCell;
pub use static_ref_shared::StaticRefShared;
