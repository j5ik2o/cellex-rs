#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types))]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::missing_errors_doc)]
#![deny(clippy::missing_panics_doc)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::redundant_field_names)]
#![deny(clippy::redundant_pattern)]
#![deny(clippy::redundant_static_lifetimes)]
#![deny(clippy::unnecessary_to_owned)]
#![deny(clippy::unnecessary_struct_initialization)]
#![deny(clippy::needless_borrow)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::manual_ok_or)]
#![deny(clippy::manual_map)]
#![deny(clippy::manual_let_else)]
#![deny(clippy::manual_strip)]
#![deny(clippy::unused_async)]
#![deny(clippy::unused_self)]
#![deny(clippy::unnecessary_wraps)]
#![deny(clippy::unreachable)]
#![deny(clippy::empty_enum)]
#![deny(clippy::no_effect)]
#![deny(dropping_copy_types)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::print_stdout)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::must_use_candidate)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![deny(clippy::clone_on_copy)]
#![deny(clippy::len_without_is_empty)]
#![deny(clippy::wrong_self_convention)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::redundant_clone))]
#![deny(clippy::from_over_into)]
#![deny(clippy::eq_op)]
#![deny(clippy::bool_comparison)]
#![deny(clippy::needless_bool)]
#![deny(clippy::match_like_matches_macro)]
#![deny(clippy::manual_assert)]
#![deny(clippy::naive_bytecount)]
#![deny(clippy::if_same_then_else)]
#![deny(clippy::cmp_null)]

//! Utilities for std runtime.
//!
//! This module binds the abstractions defined in `cellex_utils_core_rs` to tokio-based
//! implementations, providing `Arc`-based backends, synchronization primitives, and deadline
//! timers. The structure is primarily based on re-exports to avoid circular dependencies with the
//! core layer, and `TokioDeadlineTimer` is also provided from here.

/// Collection data structures tailored for std environments.
pub mod collections;
/// Concurrency primitives backed by Tokio synchronization types.
pub mod concurrent;
/// Shared ownership and state cell implementations for std environments.
pub mod sync;
/// Tokio-specific timing utilities.
pub mod timing;
/// Adaptors exposing v2 abstractions with std backends.
pub mod v2;

#[allow(deprecated)]
pub use cellex_utils_core_rs::{
  DeadlineTimer, DeadlineTimerError, DeadlineTimerExpired, DeadlineTimerKey, DeadlineTimerKeyAllocator, Element,
  MpscHandle, PriorityMessage, QueueBase, QueueError, QueueHandle, QueueReader, QueueRw, QueueRwHandle, QueueSize,
  QueueStorage, QueueWriter, RingBackend, RingBuffer, RingQueue, RingStorageBackend, Shared, SharedFactory, SharedFn,
  Stack, StackBackend, StackHandle, StackStorage, StackStorageBackend, StateCell, TimerDeadline, DEFAULT_CAPACITY,
  DEFAULT_PRIORITY, PRIORITY_LEVELS,
};
pub use v2::collections::{StdFifoSyncQueue, StdMpscSyncQueue, StdSpscSyncQueue, StdVecSyncStack};

/// Prelude module that re-exports commonly used types and traits.
pub mod prelude {
  #[allow(deprecated)]
  pub use cellex_utils_core_rs::{
    DeadlineTimer, DeadlineTimerError, DeadlineTimerExpired, DeadlineTimerKey, DeadlineTimerKeyAllocator, Element,
    MpscHandle, PriorityMessage, QueueBase, QueueError, QueueReader, QueueRw, QueueRwHandle, QueueSize, QueueStorage,
    QueueWriter, RingBackend, RingBuffer, RingStorageBackend, Shared, SharedFactory, SharedFn, Stack, StackBackend,
    StackHandle, StackStorage, StackStorageBackend, StateCell, TimerDeadline, DEFAULT_CAPACITY, DEFAULT_PRIORITY,
    PRIORITY_LEVELS,
  };

  #[allow(deprecated)]
  pub use crate::{
    collections::{
      queue::{
        mpsc::{ArcMpscBoundedQueue, ArcMpscUnboundedQueue},
        priority::ArcPriorityQueue,
        ring::ArcRingQueue,
      },
      stack::ArcStack,
    },
    concurrent::{
      AsyncBarrier, CountDownLatch, Synchronized, SynchronizedRw, TokioAsyncBarrierBackend, TokioCountDownLatchBackend,
      TokioMutexBackend, TokioRwLockBackend, TokioWaitGroupBackend, WaitGroup,
    },
    sync::{ArcShared, ArcStateCell},
    timing::TokioDeadlineTimer,
  };
}
