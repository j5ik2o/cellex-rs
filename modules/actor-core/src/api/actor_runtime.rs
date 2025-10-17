//! Actor runtime traits and type aliases.
//!
//! This module defines the high-level `ActorRuntime` trait that extends
//! `MailboxRuntime` with actor-system-specific capabilities such as
//! receive timeouts, failure handling, and metrics integration.

mod generic_actor_runtime;

pub use generic_actor_runtime::GenericActorRuntime;

use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::metrics::MetricsSinkShared;
use crate::api::supervision::escalation::FailureEventHandler;
use crate::api::supervision::escalation::FailureEventListener;
use crate::internal::mailbox::PriorityMailboxSpawnerHandle;
use crate::internal::scheduler::SchedulerBuilder;
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryProviderShared;
use crate::shared::receive_timeout::ReceiveTimeoutSchedulerFactoryShared;

/// Helper alias mapping an actor runtime to its mailbox runtime.
pub type MailboxOf<R> = <R as ActorRuntime>::MailboxFactory;

/// Helper alias mapping an actor runtime to the queue type of its mailbox runtime.
pub type MailboxQueueOf<R, M> = <MailboxOf<R> as MailboxFactory>::Queue<M>;

/// Helper alias mapping an actor runtime to the signal type of its mailbox runtime.
pub type MailboxSignalOf<R> = <MailboxOf<R> as MailboxFactory>::Signal;

/// Helper alias mapping an actor runtime to the concurrency marker of its mailbox runtime.
pub type MailboxConcurrencyOf<R> = <MailboxOf<R> as MailboxFactory>::Concurrency;

/// High-level runtime interface that extends [`MailboxFactory`] with bundle-specific capabilities.
///
/// This trait provides a facade over a mailbox runtime, adding actor-system-level
/// features such as:
/// - Receive timeout configuration
/// - Failure event listeners and escalation handlers
/// - Metrics integration
/// - Scheduler builder configuration
#[allow(dead_code)]
pub trait ActorRuntime: Clone {
  /// Underlying mailbox runtime retained by this actor runtime facade.
  type MailboxFactory: MailboxFactory + Clone + 'static;

  /// Returns a shared reference to the underlying mailbox runtime.
  fn mailbox_factory(&self) -> &Self::MailboxFactory;

  /// Consumes `self` and returns the underlying mailbox runtime.
  fn into_mailbox_factory(self) -> Self::MailboxFactory
  where
    Self: Sized;

  /// Returns the shared handle to the underlying mailbox runtime.
  fn mailbox_factory_shared(&self) -> ArcShared<Self::MailboxFactory>;

  /// Returns the receive-timeout scheduler factory configured for this runtime.
  fn receive_timeout_scheduler_factory_shared_opt(
    &self,
  ) -> Option<ReceiveTimeoutSchedulerFactoryShared<DynMessage, MailboxOf<Self>>>;

  /// Overrides the receive-timeout scheduler factory using the base mailbox runtime type.
  fn with_receive_timeout_scheduler_factory_shared(
    self,
    factory: ReceiveTimeoutSchedulerFactoryShared<DynMessage, MailboxOf<Self>>,
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
  fn root_event_listener_opt(&self) -> Option<FailureEventListener>;

  /// Overrides the root failure event listener.
  fn with_root_event_listener_opt(self, listener: Option<FailureEventListener>) -> Self
  where
    Self: Sized;

  /// Returns the root escalation handler configured for the runtime.
  fn root_escalation_handler_opt(&self) -> Option<FailureEventHandler>;

  /// Overrides the root escalation handler.
  fn with_root_escalation_handler_opt(self, handler: Option<FailureEventHandler>) -> Self
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
  fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, Self::MailboxFactory>
  where
    M: Element,
    MailboxQueueOf<Self, PriorityEnvelope<M>>: Clone,
    MailboxSignalOf<Self>: Clone;

  /// Overrides the scheduler builder used during actor system construction.
  fn with_scheduler_builder(self, builder: SchedulerBuilder<DynMessage, Self::MailboxFactory>) -> Self
  where
    Self: Sized;

  /// Returns the scheduler builder configured for this runtime.
  fn scheduler_builder_shared(&self) -> ArcShared<SchedulerBuilder<DynMessage, Self::MailboxFactory>>;

  /// Overrides the scheduler builder using a shared handle.
  fn with_scheduler_builder_shared(
    self,
    builder: ArcShared<SchedulerBuilder<DynMessage, Self::MailboxFactory>>,
  ) -> Self
  where
    Self: Sized;
}
