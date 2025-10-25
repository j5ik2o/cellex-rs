use cellex_actor_core_rs::api::{
  actor_runtime::GenericActorRuntime, receive_timeout::ReceiveTimeoutSchedulerFactoryProviderShared,
};

use crate::{scheduler::tokio_scheduler::tokio_scheduler_builder, TokioMailboxFactory, TokioReceiveTimeoutDriver};

/// Extension trait that installs Tokio-specific scheduler and timeout settings on
/// [`GenericActorRuntime`].
pub trait TokioActorRuntimeExt {
  /// Replaces the scheduler with the Tokio-backed implementation.
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxFactory>;
}

impl TokioActorRuntimeExt for GenericActorRuntime<TokioMailboxFactory> {
  fn with_tokio_scheduler(self) -> GenericActorRuntime<TokioMailboxFactory> {
    self.with_scheduler_builder(tokio_scheduler_builder()).with_receive_timeout_scheduler_factory_provider_shared_opt(
      Some(ReceiveTimeoutSchedulerFactoryProviderShared::new(TokioReceiveTimeoutDriver::new())),
    )
  }
}
