//! no_std-friendly queue primitives shared between runtimes.

mod queue_size;
/// Queue trait definitions shared across all backends.
pub mod traits;

/// Multi-producer/single-consumer queue abstractions.
pub mod mpsc;
/// Priority-ordered queue abstractions.
pub mod priority;
/// Ring-buffer-based queue implementations and utilities.
pub mod ring;

pub use queue_size::QueueSize;
