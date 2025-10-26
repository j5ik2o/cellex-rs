//! Storage layer abstractions for queue backends.

mod queue_storage;
mod vec_ring_storage;

#[allow(unused_imports)]
pub use queue_storage::QueueStorage;
#[allow(unused_imports)]
pub use vec_ring_storage::VecRingStorage;
