//! Queue facade exposed to collection users.

mod queue;

pub use queue::{FifoQueue, MpscQueue, PriorityQueue, Queue, SpscQueue};

#[cfg(test)]
mod tests;
