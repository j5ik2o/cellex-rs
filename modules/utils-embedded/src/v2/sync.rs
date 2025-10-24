//! Embedded-specific synchronization adaptors for v2 abstractions.

#[cfg(feature = "embassy")]
mod embassy_support;

#[cfg(feature = "embassy")]
pub use embassy_support::{EmbassyAsyncMutex, EmbassyAsyncMutexGuard};
