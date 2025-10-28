//! Concurrency primitives module.
//!
//! This module provides concurrency and synchronization primitives that can be used in `no_std`
//! environments. You can select between `Rc` or `Arc` based implementations via feature flags.
//!
//! # Provided Synchronization Primitives
//!
//! - **AsyncBarrier**: Barrier for multiple tasks to wait at synchronization points
//! - **CountDownLatch**: Latch that waits until count reaches 0
//! - **WaitGroup**: Wait group for tracking completion of multiple tasks
//! - **Synchronized**: Exclusive access control (Mutex-based)
//! - **SynchronizedRw**: Read/write access control (RwLock-based)
//!
//! # Feature Flags
//!
//! - **`rc`**: `Rc`-based implementation (single-threaded only)
//! - **`arc`**: `Arc`-based implementation (multi-threaded support)
//!   - `ArcLocal*`: `Arc` + `LocalMutex`/`LocalRwLock` (single-thread optimized)
//!   - `ArcCs*`: `Arc` + `CsMutex`/`CsRwLock` (critical section-based)
//!   - `Arc*`: `Arc` + standard Mutex/RwLock

/// `Rc`-based async barrier implementation.
#[cfg(feature = "rc")]
mod rc_async_barrier;
/// `Rc`-based countdown latch implementation.
#[cfg(feature = "rc")]
mod rc_count_down_latch;
/// `Rc`-based synchronized type implementations.
#[cfg(feature = "rc")]
pub mod rc_synchronized;
/// `Rc`-based wait group implementation.
#[cfg(feature = "rc")]
mod rc_wait_group;

#[cfg(feature = "rc")]
pub use rc_async_barrier::*;
#[cfg(feature = "rc")]
pub use rc_count_down_latch::*;
#[cfg(feature = "rc")]
pub use rc_wait_group::*;

/// `Arc`-based async barrier implementation.
#[cfg(feature = "arc")]
mod arc_async_barrier;
/// `Arc`-based countdown latch implementation.
#[cfg(feature = "arc")]
mod arc_count_down_latch;
/// `Arc`-based synchronized type implementations.
#[cfg(feature = "arc")]
pub mod arc_synchronized;
/// `Arc`-based wait group implementation.
#[cfg(feature = "arc")]
mod arc_wait_group;

#[cfg(feature = "arc")]
pub use arc_async_barrier::*;
#[cfg(feature = "arc")]
pub use arc_count_down_latch::*;
#[cfg(feature = "arc")]
pub use arc_wait_group::*;
