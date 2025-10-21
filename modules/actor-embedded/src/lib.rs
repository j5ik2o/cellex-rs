//! Actor system implementation for embedded environments.
//!
//! This crate provides implementations for running actor systems in `no_std` environments.
//! Supports local mailboxes, Arc-based mailboxes, Embassy integration, and more.

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
#![no_std]

extern crate alloc;

#[cfg(feature = "embedded_arc")]
mod arc_mailbox;
#[cfg(feature = "embedded_arc")]
mod arc_priority_mailbox;
mod local_mailbox;
#[cfg(feature = "embassy_executor")]
mod receive_timeout;
/// Runtime driver for failure event handling in embedded environments.
pub mod runtime_driver;
#[cfg(feature = "embassy_executor")]
mod scheduler;
mod spawn;
mod timer;

#[cfg(feature = "embedded_arc")]
pub use arc_mailbox::{ArcMailbox, ArcMailboxRuntime, ArcMailboxSender};
#[cfg(feature = "embedded_arc")]
pub use arc_priority_mailbox::{ArcPriorityMailbox, ArcPriorityMailboxRuntime, ArcPriorityMailboxSender};
#[cfg(feature = "embedded_arc")]
pub use cellex_utils_embedded_rs::sync::{ArcCsStateCell, ArcLocalStateCell, ArcShared, ArcStateCell};
#[cfg(feature = "embedded_rc")]
pub use cellex_utils_embedded_rs::sync::{RcShared, RcStateCell};
#[cfg(feature = "embassy_executor")]
mod embassy_dispatcher;

pub use local_mailbox::{LocalMailbox, LocalMailboxRuntime, LocalMailboxSender};
#[cfg(feature = "embassy_executor")]
pub use receive_timeout::EmbassyReceiveTimeoutSchedulerFactory;
#[cfg(feature = "embassy_executor")]
pub use scheduler::{embassy_scheduler_builder, EmbassyActorRuntimeExt, EmbassyScheduler};
pub use spawn::ImmediateSpawner;
pub use timer::ImmediateTimer;

/// Prelude that re-exports commonly used types in embedded environments.
pub mod prelude {
  #[cfg(feature = "embedded_arc")]
  pub use super::{
    ArcCsStateCell, ArcLocalStateCell, ArcMailbox, ArcMailboxRuntime, ArcMailboxSender, ArcPriorityMailbox,
    ArcPriorityMailboxRuntime, ArcPriorityMailboxSender, ArcShared, ArcStateCell,
  };
  #[cfg(feature = "embassy_executor")]
  pub use super::{EmbassyActorRuntimeExt, EmbassyScheduler};
  pub use super::{ImmediateSpawner, ImmediateTimer, LocalMailbox, LocalMailboxRuntime, LocalMailboxSender};
  #[cfg(feature = "embedded_rc")]
  pub use super::{RcShared, RcStateCell};
}

#[cfg(test)]
mod tests;

/// Default actor runtime preset for Embassy-based environments.
#[cfg(feature = "embassy_executor")]
pub type EmbassyActorRuntime = cellex_actor_core_rs::GenericActorRuntime<LocalMailboxRuntime>;

/// Builds the default Embassy-oriented actor runtime preset using the provided spawner.
#[cfg(feature = "embassy_executor")]
#[must_use]
pub fn embassy_actor_runtime(spawner: &'static embassy_executor::Spawner) -> EmbassyActorRuntime {
  use scheduler::EmbassyActorRuntimeExt;

  cellex_actor_core_rs::GenericActorRuntime::new(LocalMailboxRuntime::default()).with_embassy_scheduler(spawner)
}
