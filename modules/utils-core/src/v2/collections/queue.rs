//! Queue abstractions rebuilt for the v2 collections layer.

pub mod backend;
pub mod capabilities;
pub mod facade;
mod storage;
pub mod type_keys;

pub use backend::{OfferOutcome, OverflowPolicy, PriorityBackend, QueueBackend, QueueError};
pub use capabilities::{MultiProducer, SingleConsumer, SingleProducer, SupportsPeek};
pub use facade::{FifoQueue, MpscQueue, PriorityQueue, Queue, SpscQueue};
pub use storage::QueueStorage;
pub use type_keys::{FifoKey, MpscKey, PriorityKey, SpscKey, TypeKey};
