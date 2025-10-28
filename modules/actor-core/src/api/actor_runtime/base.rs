use cellex_utils_core_rs::{
  collections::Element,
  sync::{async_mutex_like::AsyncMutexLike, sync_mutex_like::SyncMutexLike, ArcShared},
};

use crate::{
  api::{
    actor_scheduler::ActorSchedulerHandleBuilder,
    failure::failure_event_stream::FailureEventListener,
    metrics::MetricsSinkShared,
    receive_timeout::{ReceiveTimeoutSchedulerFactoryProviderShared, ReceiveTimeoutSchedulerFactoryShared},
  },
  internal::mailbox::PriorityMailboxSpawnerHandle,
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory},
    messaging::AnyMessage,
    supervision::FailureEventHandler,
  },
};

/// Helper alias mapping an actor runtime to its associated [`MailboxFactory`].
pub type MailboxOf<R> = <R as ActorRuntime>::MailboxFactory;

/// Helper alias that resolves the queue type produced by the runtime's mailbox factory.
pub type MailboxQueueOf<R, M> = <MailboxOf<R> as MailboxFactory>::Queue<M>;

/// Helper alias that resolves the signal type produced by the runtime's mailbox factory.
pub type MailboxSignalOf<R> = <MailboxOf<R> as MailboxFactory>::Signal;

/// Helper alias that resolves the concurrency marker of the runtime's mailbox factory.
pub type MailboxConcurrencyOf<R> = <MailboxOf<R> as MailboxFactory>::Concurrency;

/// High-level runtime interface that extends [`MailboxFactory`] with bundle-specific capabilities.
///
/// This trait provides a facade over a [`MailboxFactory`], adding actor-system-level features such
/// as:
/// - Receive timeout configuration
/// - Failure event listeners and escalation handlers
/// - Metrics integration
/// - Scheduler builder configuration
#[allow(dead_code)]
pub trait ActorRuntime: Clone {
  /// Underlying mailbox factory retained by this actor runtime facade.
  type MailboxFactory: MailboxFactory + Clone + 'static;

  /// Synchronous mutex type provided by this runtime.
  type SyncMutex<T>: SyncMutexLike<T>;

  /// Asynchronous mutex type provided by this runtime.
  ///
  /// Note: `T` must implement `Send` for async mutex implementations that
  /// require cross-thread safety (e.g., Tokio). For no_std environments,
  /// this bound may be relaxed.
  type AsyncMutex<T: Send>: AsyncMutexLike<T>;

  /// Returns a shared reference to the underlying mailbox factory.
  fn mailbox_factory(&self) -> &Self::MailboxFactory;

  /// Consumes `self` and returns the underlying mailbox factory.
  fn into_mailbox_factory(self) -> Self::MailboxFactory
  where
    Self: Sized;

  /// Returns the shared handle to the underlying mailbox factory.
  fn mailbox_factory_shared(&self) -> ArcShared<Self::MailboxFactory>;

  /// Returns the receive-timeout scheduler factory configured for this runtime.
  fn receive_timeout_scheduler_factory_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<Self>>>;

  /// Overrides the receive-timeout scheduler factory using the base mailbox factory.
  fn with_receive_timeout_scheduler_factory_shared(
    self,
    factory: ReceiveTimeoutSchedulerFactoryShared<AnyMessage, MailboxOf<Self>>,
  ) -> Self
  where
    Self: Sized;

  /// Returns the receive-timeout scheduler factory provider configured for this runtime.
  fn receive_timeout_scheduler_factory_provider_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryProviderShared<Self::MailboxFactory>>;

  /// Overrides the receive-timeout scheduler factory provider.
  fn with_receive_timeout_scheduler_factory_provider_shared_opt(
    self,
    driver: Option<ReceiveTimeoutSchedulerFactoryProviderShared<Self::MailboxFactory>>,
  ) -> Self
  where
    Self: Sized;

  /// Returns the root failure event listener configured for the runtime.
  fn root_failure_event_listener_opt(&self) -> Option<FailureEventListener>;

  /// Overrides the root failure event listener.
  fn with_root_failure_event_listener_opt(self, listener: Option<FailureEventListener>) -> Self
  where
    Self: Sized;

  /// Returns the root escalation handler configured for the runtime.
  fn root_escalation_failure_event_handler_opt(&self) -> Option<FailureEventHandler>;

  /// Overrides the root escalation handler.
  fn with_root_escalation_failure_event_handler_opt(self, handler: Option<FailureEventHandler>) -> Self
  where
    Self: Sized;

  /// Returns the metrics sink configured for the runtime.
  fn metrics_sink_shared_opt(&self) -> Option<MetricsSinkShared>;

  /// Overrides the metrics sink.
  fn with_metrics_sink_shared_opt(self, sink: Option<MetricsSinkShared>) -> Self
  where
    Self: Sized;

  /// Overrides the metrics sink using a shared handle.
  fn with_metrics_sink_shared(self, sink: MetricsSinkShared) -> Self
  where
    Self: Sized;

  /// Returns a priority mailbox spawner handle without exposing the internal factory.
  fn priority_mailbox_spawner_handle<M>(&self) -> PriorityMailboxSpawnerHandle<M, Self::MailboxFactory>
  where
    M: Element,
    MailboxQueueOf<Self, PriorityEnvelope<M>>: Clone,
    MailboxSignalOf<Self>: Clone;

  /// Overrides the scheduler builder used during actor system construction.
  fn with_actor_scheduler_handle_builder(self, builder: ActorSchedulerHandleBuilder<Self::MailboxFactory>) -> Self
  where
    Self: Sized;

  /// Returns the scheduler builder configured for this runtime.
  fn scheduler_builder_shared_builder_shared(&self) -> ArcShared<ActorSchedulerHandleBuilder<Self::MailboxFactory>>;

  /// Overrides the scheduler builder using a shared handle.
  fn with_scheduler_builder_shared_builder_shared(
    self,
    builder: ArcShared<ActorSchedulerHandleBuilder<Self::MailboxFactory>>,
  ) -> Self
  where
    Self: Sized;
}
