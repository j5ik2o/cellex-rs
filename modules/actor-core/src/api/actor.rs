//! Actor API aggregation module.

/// Actor failure information
pub mod actor_failure;
mod actor_id;
mod actor_path;
/// Actor reference types
pub mod actor_ref;
/// Ask pattern for request-response communication
pub mod ask;
/// Actor behavior definitions
pub mod behavior;
mod child_naming;
/// Actor execution context
pub mod context;
/// Actor spawn properties
mod props;
/// Root context for top-level actors
pub mod root_context;
/// Shutdown coordination
pub mod shutdown_token;
/// Actor lifecycle signals
pub mod signal;
mod spawn;
mod spawn_error;
#[cfg(test)]
mod tests;
mod timer;

pub use actor_id::ActorId;
pub use actor_path::ActorPath;
pub use child_naming::ChildNaming;
pub use props::Props;
pub use spawn::Spawn;
pub use spawn_error::SpawnError;
pub use timer::Timer;
