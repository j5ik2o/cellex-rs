//! Queue facade exposed to collection users.

mod mpsc_consumer;
mod mpsc_producer;
mod queue;
mod spsc_consumer;
mod spsc_producer;

pub use mpsc_consumer::MpscConsumer;
pub use mpsc_producer::MpscProducer;
pub use queue::{FifoQueue, MpscQueue, PriorityQueue, Queue, SpscQueue};
pub use spsc_consumer::SpscConsumer;
pub use spsc_producer::SpscProducer;

#[cfg(test)]
mod tests;
