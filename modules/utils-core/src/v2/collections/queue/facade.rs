//! Queue facade exposed to collection users.

mod async_mpsc_consumer;
mod async_mpsc_producer;
mod async_queue;
mod async_spsc_consumer;
mod async_spsc_producer;
mod mpsc_consumer;
mod mpsc_producer;
mod queue;
mod spsc_consumer;
mod spsc_producer;

pub use async_mpsc_consumer::AsyncMpscConsumer;
pub use async_mpsc_producer::AsyncMpscProducer;
pub use async_queue::{AsyncFifoQueue, AsyncMpscQueue, AsyncPriorityQueue, AsyncQueue, AsyncSpscQueue};
pub use async_spsc_consumer::AsyncSpscConsumer;
pub use async_spsc_producer::AsyncSpscProducer;
pub use mpsc_consumer::MpscConsumer;
pub use mpsc_producer::MpscProducer;
pub use queue::{FifoQueue, MpscQueue, PriorityQueue, Queue, SpscQueue};
pub use spsc_consumer::SpscConsumer;
pub use spsc_producer::SpscProducer;

#[cfg(test)]
mod tests;
