//! v2 abstractions specialized for embedded targets.

#[cfg(feature = "embassy")]
#[doc = "Collection utilities for v2 Embassy integrations."]
pub mod collections;

#[cfg(feature = "embassy")]
/// Synchronization primitives for v2 Embassy integrations.
pub mod sync;
