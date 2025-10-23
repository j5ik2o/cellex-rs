//! Queue abstractions rebuilt for the v2 collections layer.

mod async_mpsc_consumer;
mod async_mpsc_producer;
mod async_queue;
mod async_spsc_consumer;
mod async_spsc_producer;
pub mod backend;
pub mod capabilities;
mod mpsc_consumer;
mod mpsc_producer;
mod queue_api;
mod spsc_consumer;
mod spsc_producer;
mod storage;
pub mod type_keys;

pub use async_mpsc_consumer::AsyncMpscConsumer;
pub use async_mpsc_producer::AsyncMpscProducer;
pub use async_queue::{AsyncFifoQueue, AsyncMpscQueue, AsyncPriorityQueue, AsyncQueue, AsyncSpscQueue};
pub use async_spsc_consumer::AsyncSpscConsumer;
pub use async_spsc_producer::AsyncSpscProducer;
pub use backend::{
  AsyncPriorityBackend, AsyncQueueBackend, OfferOutcome, OverflowPolicy, PriorityBackend, QueueBackend, QueueError,
  SyncAdapterQueueBackend, VecRingBackend,
};
pub use capabilities::{MultiProducer, SingleConsumer, SingleProducer, SupportsPeek};
pub use mpsc_consumer::MpscConsumer;
pub use mpsc_producer::MpscProducer;
pub use queue_api::{FifoQueue, MpscQueue, PriorityQueue, Queue, SpscQueue};
pub use spsc_consumer::SpscConsumer;
pub use spsc_producer::SpscProducer;
pub use storage::{QueueStorage, VecRingStorage};
pub use type_keys::{FifoKey, MpscKey, PriorityKey, SpscKey, TypeKey};

#[cfg(test)]
mod tests;

/// Default shared queue alias backed by [`VecRingBackend`].
pub type SharedVecRingQueue<T, K = FifoKey> = Queue<T, K, VecRingBackend<T>>;

/// Default async shared queue alias backed by [`VecRingBackend`] via the sync adapter.
pub type AsyncSharedVecRingQueue<T, K = FifoKey> = AsyncQueue<T, K, SyncAdapterQueueBackend<T, VecRingBackend<T>>>;
