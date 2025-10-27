//! `TokioReceiveTimeoutSchedulerFactory` implementation.

use cellex_actor_core_rs::{
  api::receive_timeout::{ReceiveTimeoutScheduler, ReceiveTimeoutSchedulerFactory},
  shared::messaging::{AnyMessage, MapSystemShared},
};

use super::{shared::TokioSender, tokio_receive_timeout_scheduler::TokioReceiveTimeoutScheduler};
use crate::tokio_mailbox::TokioMailboxFactory;

/// `ReceiveTimeoutSchedulerFactory` implementation for Tokio runtime.
///
/// Receives the priority mailbox producer and SystemMessage conversion closure,
/// spawns an internal scheduler task, and returns a `ReceiveTimeoutScheduler`.
/// Assigning it via `GenericActorSystemConfig::with_receive_timeout_scheduler_factory_shared_opt`
/// or `GenericActorSystemConfig::set_receive_timeout_scheduler_factory_shared_opt` enables
/// `ReceiveTimeout` support for the Tokio runtime.
pub struct TokioReceiveTimeoutSchedulerFactory;

impl TokioReceiveTimeoutSchedulerFactory {
  /// Creates a new factory.
  #[must_use]
  pub const fn new() -> Self {
    Self
  }
}

impl Default for TokioReceiveTimeoutSchedulerFactory {
  fn default() -> Self {
    Self::new()
  }
}

impl ReceiveTimeoutSchedulerFactory<AnyMessage, TokioMailboxFactory> for TokioReceiveTimeoutSchedulerFactory {
  fn create(&self, sender: TokioSender, map_system: MapSystemShared<AnyMessage>) -> Box<dyn ReceiveTimeoutScheduler> {
    Box::new(TokioReceiveTimeoutScheduler::new(sender, map_system))
  }
}
