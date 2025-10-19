//! Queue variants available in the std build.

/// Tokio-backed MPSC queue implementations.
pub mod mpsc;
/// Priority queue wrappers built on ring queues.
pub mod priority;
/// Ring buffer queue adapters for std environments.
pub mod ring;
