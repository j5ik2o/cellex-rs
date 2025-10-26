//! Queue abstractions rebuilt for the v2 collections layer.

mod async_mpsc_consumer;
mod async_mpsc_producer;
mod async_queue;
mod async_spsc_consumer;
mod async_spsc_producer;
pub mod backend;
pub mod capabilities;
pub mod storage;
mod sync_mpsc_consumer;
mod sync_mpsc_producer;
mod sync_queue;
mod sync_spsc_consumer;
mod sync_spsc_producer;
pub mod type_keys;

pub use async_mpsc_consumer::AsyncMpscConsumer;
pub use async_mpsc_producer::AsyncMpscProducer;
pub use async_queue::{AsyncFifoQueue, AsyncMpscQueue, AsyncPriorityQueue, AsyncQueue, AsyncSpscQueue};
pub use async_spsc_consumer::AsyncSpscConsumer;
pub use async_spsc_producer::AsyncSpscProducer;
pub use sync_mpsc_consumer::SyncMpscConsumer;
pub use sync_mpsc_producer::SyncMpscProducer;
pub use sync_queue::{FifoQueue, MpscQueue, PriorityQueue, SpscQueue, SyncQueue};
pub use sync_spsc_consumer::SyncSpscConsumer;
pub use sync_spsc_producer::SyncSpscProducer;

#[cfg(test)]
mod tests;

/// Default shared queue alias backed by [`backend::VecRingBackend`].
pub type SharedVecRingQueue<T, K = type_keys::FifoKey> = SyncQueue<T, K, backend::VecRingBackend<T>>;

/// Default async shared queue alias backed by [`backend::VecRingBackend`] via the sync adapter.
pub type AsyncSharedVecRingQueue<T, K = type_keys::FifoKey> =
  AsyncQueue<T, K, backend::SyncAdapterQueueBackend<T, backend::VecRingBackend<T>>>;
