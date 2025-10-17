//! Actor API aggregation module.

/// Actor reference types
pub mod actor_ref;
/// Ask pattern for request-response communication
pub mod ask;
/// Actor behavior definitions
pub mod behavior;
/// Actor execution context
pub mod context;
/// Actor failure information
pub mod failure;
/// Actor spawn properties
pub mod props;
/// Root context for top-level actors
pub mod root_context;
/// Shutdown coordination
pub mod shutdown_token;
/// Actor lifecycle signals
pub mod signal;
#[cfg(test)]
mod tests;
