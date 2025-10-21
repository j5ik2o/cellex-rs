//! Priority queue implementation

#[cfg(feature = "arc")]
/// `Arc`-based priority queue
mod arc_priority_queue;
#[cfg(feature = "rc")]
/// `Rc`-based priority queue
mod rc_priority_queue;

#[cfg(feature = "arc")]
pub use arc_priority_queue::{ArcCsPriorityQueue, ArcLocalPriorityQueue, ArcPriorityQueue};
#[cfg(feature = "rc")]
pub use rc_priority_queue::RcPriorityQueue;
