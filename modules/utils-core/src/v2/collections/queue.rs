//! Queue abstractions rebuilt for the v2 collections layer.

pub mod backend;
pub mod capabilities;
pub mod facade;
mod storage;
pub mod type_keys;

pub use backend::{OfferOutcome, OverflowPolicy, PriorityBackend, QueueBackend, QueueError, VecRingBackend};
pub use capabilities::{MultiProducer, SingleConsumer, SingleProducer, SupportsPeek};
pub use facade::{
  FifoQueue, MpscConsumer, MpscProducer, MpscQueue, PriorityQueue, Queue, SpscConsumer, SpscProducer, SpscQueue,
};
pub use storage::{QueueStorage, VecRingStorage};
pub use type_keys::{FifoKey, MpscKey, PriorityKey, SpscKey, TypeKey};

/// Default shared queue alias backed by [`VecRingBackend`].
pub type SharedVecRingQueue<T, K = FifoKey> = Queue<T, K, VecRingBackend<T>>;
