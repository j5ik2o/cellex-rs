//! v2 abstractions specialized for embedded targets.

#[cfg(feature = "embassy")]
#[doc = "Collection utilities for v2 Embassy integrations."]
pub mod collections;

mod sync;

#[cfg(feature = "embassy")]
pub use sync::{EmbassyAsyncMutex, EmbassyAsyncMutexGuard};
