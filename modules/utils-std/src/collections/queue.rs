//! Queue variants available in the std build.

/// Tokio-backed MPSC queue implementations.
pub mod mpsc;
mod mutex_mpsc_buffer_storage;
mod mutex_ring_buffer_storage;
/// Priority queue wrappers built on ring queues.
pub mod priority;
/// Ring buffer queue adapters for std environments.
pub mod ring;

pub(crate) use mutex_mpsc_buffer_storage::MutexMpscBufferStorage;
pub(crate) use mutex_ring_buffer_storage::MutexRingBufferStorage;
