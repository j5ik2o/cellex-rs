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

//! Utility library for embedded environments.
//!
//! This crate provides concurrency and synchronization primitives that work in `no_std`
//! environments. It is designed to enable runtimes like `actor-embedded` to operate without the
//! standard library.
//!
//! # Key Features
//!
//! ## Synchronization Primitives
//!
//! - **AsyncBarrier**: Barrier mechanism for multiple tasks to wait at synchronization points
//! - **CountDownLatch**: Countdown-based latch (waits until count reaches 0)
//! - **WaitGroup**: Wait group for tracking completion of multiple tasks
//! - **Synchronized**: Synchronization type providing exclusive access control (Mutex-based)
//! - **SynchronizedRw**: Synchronization type providing read/write access control (RwLock-based)
//!
//! ## Collections
//!
//! - **Queue**: Bounded/unbounded queues, priority queues, ring buffers
//! - **Stack**: Stack data structure
//! - **MPSC**: Multi-producer, single-consumer queues
//!
//! ## Timers
//!
//! - **ManualDeadlineTimer**: Software-stepped deadline timer
//!
//! # Ownership Models
//!
//! The ownership model can be switched via feature flags:
//!
//! - **`rc` feature**: `Rc`-based implementation (single-threaded, default)
//! - **`arc` feature**: `Arc`-based implementation (multi-threaded support)
//!
//! # Embassy Integration
//!
//! This crate integrates with the [Embassy](https://embassy.dev/) ecosystem and
//! internally uses `embassy_sync` synchronization primitives.

#![no_std]

extern crate alloc;

#[cfg(test)]
mod tests;

/// Collection utilities for embedded environments.
#[cfg(feature = "embassy")]
pub mod collections;
/// Concurrency primitives specialized for embedded runtimes.
pub mod concurrent;
/// Synchronization and shared-state helpers for embedded systems.
pub mod sync;
/// Timer helpers for embedded environments.
pub mod timing;
