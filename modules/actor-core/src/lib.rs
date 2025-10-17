//! nexus-actor-rs core library
//!
//! Core module of the actor model library implemented in Rust.
//! Provides type-safe message passing, supervisor hierarchies,
//! and Akka/Pekko Typed-style Behavior API.
//!
//! # Key Features
//! - Typed actor references (`ActorRef<U, R>`)
//! - Behavior DSL (Akka Typed-style)
//! - Supervisor strategies
//! - Ask pattern (Request-Response)
//! - Mailboxes and dispatchers
//! - Event stream
//!
//! # Example Usage
//! ```ignore
//! use cellex_actor_core_rs::*;
//!
//! let behavior = Behaviors::receive(|ctx, msg: String| {
//!     println!("Received: {}", msg);
//!     Ok(Behaviors::same())
//! });
//! ```

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
#![deny(clippy::from_over_into)]
#![deny(clippy::eq_op)]
#![deny(clippy::bool_comparison)]
#![deny(clippy::needless_bool)]
#![deny(clippy::match_like_matches_macro)]
#![deny(clippy::manual_assert)]
#![deny(clippy::naive_bytecount)]
#![deny(clippy::if_same_then_else)]
#![deny(clippy::cmp_null)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::result_large_err)]

#[cfg(feature = "alloc")]
extern crate alloc;

use crate::api::mailbox::SystemMessage;
use crate::api::messaging::DynMessage;
use cellex_utils_core_rs::QueueError;
use core::time::Duration;

/// Public API for actors
pub mod api;
/// Internal implementation details
pub mod internal;
/// Shared utilities and types
pub mod shared;

/// Marker trait capturing the synchronization guarantees required by runtime-dependent types.
#[cfg(target_has_atomic = "ptr")]
pub trait RuntimeBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> RuntimeBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
/// Marker trait for single-threaded targets without atomic pointer support.
pub trait RuntimeBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> RuntimeBound for T {}

/// Function type alias for converting system messages to message type.
#[cfg(target_has_atomic = "ptr")]
pub type MapSystemFn<M> = dyn Fn(SystemMessage) -> M + Send + Sync;

/// Function type alias for converting system messages on non-atomic targets.
#[cfg(not(target_has_atomic = "ptr"))]
pub type MapSystemFn<M> = dyn Fn(SystemMessage) -> M;

/// Minimal actor loop implementation.
///
/// Receives messages and passes them to the handler for processing.
/// Reference implementation shared by both std and embedded runtimes.
///
/// # Arguments
/// * `mailbox` - Mailbox to receive messages from
/// * `timer` - Timer used for waiting
/// * `handler` - Handler function to process messages
pub async fn actor_loop<M, MB, T, F>(mailbox: &MB, timer: &T, mut handler: F)
where
  MB: api::mailbox::Mailbox<M>,
  T: api::actor_system::Timer,
  F: FnMut(M), {
  loop {
    match mailbox.recv().await {
      Ok(message) => handler(message),
      Err(QueueError::Disconnected) => break,
      Err(QueueError::Closed(message)) => handler(message),
      Err(_) => break,
    }
    timer.sleep(Duration::from_millis(0)).await;
  }
}

#[cfg(test)]
mod tests;
