#[cfg(feature = "embassy")]
#[doc = "Async queue adapters for Embassy runtimes."]
mod async_queue;

#[cfg(feature = "embassy")]
pub use async_queue::{
  make_embassy_mpsc_queue, make_embassy_mpsc_queue_with_mutex, EmbassyBoundedMpscBackend, EmbassyCsMpscQueue,
  EmbassyMpscQueue,
};
