//! MPSC (Multiple Producer, Single Consumer) queue implementation

#[cfg(feature = "arc")]
/// `Arc`-based MPSC bounded/unbounded queue
mod arc_mpsc_bounded_queue;
#[cfg(feature = "arc")]
/// `Arc`-based MPSC unbounded queue
mod arc_mpsc_unbounded_queue;
#[cfg(feature = "rc")]
/// `Rc`-based MPSC bounded queue
mod rc_mpsc_bounded_queue;
#[cfg(feature = "rc")]
/// `Rc`-based MPSC unbounded queue
mod rc_mpsc_unbounded_queue;

#[cfg(feature = "arc")]
pub use arc_mpsc_bounded_queue::{ArcCsMpscBoundedQueue, ArcLocalMpscBoundedQueue, ArcMpscBoundedQueue};
#[cfg(feature = "arc")]
pub use arc_mpsc_unbounded_queue::{ArcCsMpscUnboundedQueue, ArcLocalMpscUnboundedQueue, ArcMpscUnboundedQueue};
#[cfg(feature = "rc")]
pub use rc_mpsc_bounded_queue::RcMpscBoundedQueue;
#[cfg(feature = "rc")]
pub use rc_mpsc_unbounded_queue::RcMpscUnboundedQueue;
