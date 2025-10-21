//! `TokioReceiveTimeoutDriver` implementation.

use cellex_actor_core_rs::{
  api::receive_timeout::{ReceiveTimeoutSchedulerFactoryProvider, ReceiveTimeoutSchedulerFactoryShared},
  shared::messaging::AnyMessage,
};

use super::tokio_receive_timeout_scheduler_factory::TokioReceiveTimeoutSchedulerFactory;
use crate::TokioMailboxRuntime;

/// Runtime driver that provisions Tokio receive-timeout factories on demand.
#[derive(Debug, Default, Clone)]
pub struct TokioReceiveTimeoutDriver;

impl TokioReceiveTimeoutDriver {
  /// Creates a new driver instance.
  #[must_use]
  pub const fn new() -> Self {
    Self
  }
}

impl ReceiveTimeoutSchedulerFactoryProvider<TokioMailboxRuntime> for TokioReceiveTimeoutDriver {
  fn build_factory(&self) -> ReceiveTimeoutSchedulerFactoryShared<AnyMessage, TokioMailboxRuntime> {
    ReceiveTimeoutSchedulerFactoryShared::new(TokioReceiveTimeoutSchedulerFactory::new())
  }
}
