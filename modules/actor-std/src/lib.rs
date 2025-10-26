//! A crate that provides actor system implementation for the Tokio asynchronous runtime.
//!
//! This crate provides components such as mailboxes, timers, and spawners
//! that run on the Tokio runtime, making the functionality of `nexus-actor-core-rs`
//! available in standard asynchronous runtime environments.

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

/// A failure event bridge module utilizing Tokio's broadcast channel.
#[cfg(any(feature = "rt-multi-thread", feature = "rt-current-thread"))]
mod failure_event_bridge;
mod failure_event_hub;
mod receive_timeout;
mod runtime_driver;
mod scheduler;
mod spawn;
mod timer;
mod tokio_mailbox;
mod tokio_priority_mailbox;

#[cfg(test)]
mod tests;

use cellex_actor_core_rs::api::actor_runtime::GenericActorRuntime;
pub use cellex_utils_std_rs::{
  sync::{ArcShared, ArcStateCell},
  Shared, SharedFactory, SharedFn,
};
pub use failure_event_hub::{FailureEventHub, FailureEventSubscription};
pub use receive_timeout::{TokioReceiveTimeoutDriver, TokioReceiveTimeoutSchedulerFactory};
pub use runtime_driver::TokioSystemHandle;
pub use scheduler::{tokio_scheduler_builder, TokioActorRuntimeExt, TokioScheduler};
pub use spawn::TokioSpawner;
pub use timer::TokioTimer;
pub use tokio_mailbox::{TokioMailbox, TokioMailboxFactory, TokioMailboxSender};
pub use tokio_priority_mailbox::{TokioPriorityMailbox, TokioPriorityMailboxFactory, TokioPriorityMailboxSender};

/// A prelude module that provides commonly used re-exported types and traits.
pub mod prelude {
  pub use cellex_actor_core_rs::actor_loop;

  pub use super::{
    ArcShared, ArcStateCell, Shared, SharedFactory, SharedFn, TokioMailbox, TokioMailboxFactory, TokioMailboxSender,
    TokioPriorityMailbox, TokioPriorityMailboxFactory, TokioPriorityMailboxSender, TokioScheduler, TokioSpawner,
    TokioSystemHandle, TokioTimer,
  };
}
/// Default actor runtime preset for Tokio environments.
pub type TokioActorRuntime = GenericActorRuntime<TokioMailboxFactory>;

/// Builds the default Tokio-oriented actor runtime preset.
#[must_use]
pub fn tokio_actor_runtime() -> TokioActorRuntime {
  use scheduler::TokioActorRuntimeExt;

  GenericActorRuntime::new(TokioMailboxFactory).with_tokio_scheduler()
}
