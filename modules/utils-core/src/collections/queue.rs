//! no_std-friendly queue primitives shared between runtimes.
//!
//! v1 queue implementations have been removed. Use v2 implementations instead.
//! This module only retains QueueSize and priority constants/traits for backward compatibility.

/// Priority-ordered queue abstractions.
pub mod priority;
mod queue_size;

pub use queue_size::QueueSize;
