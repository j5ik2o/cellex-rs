//! Actor runtime traits and type aliases.
//!
//! This module defines the high-level `ActorRuntime` trait that extends
//! `MailboxRuntime` with actor-system-specific capabilities such as
//! receive timeouts, failure handling, and metrics integration.

mod generic_runtime;

pub use generic_runtime::GenericActorRuntime;

use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

use crate::internal::mailbox::traits::MailboxRuntime;
use crate::internal::mailbox::PriorityMailboxSpawnerHandle;
use crate::internal::message::DynMessage;
use crate::internal::metrics::MetricsSinkShared;
use crate::internal::scheduler::SchedulerBuilder;
use crate::{
  FailureEventHandler, FailureEventListener, PriorityEnvelope, ReceiveTimeoutDriverShared, ReceiveTimeoutFactoryShared,
};

/// Helper alias mapping an actor runtime to its mailbox runtime.
pub type MailboxOf<R> = <R as ActorRuntime>::Mailbox;

/// Helper alias mapping an actor runtime to the queue type of its mailbox runtime.
pub type MailboxQueueOf<R, M> = <MailboxOf<R> as MailboxRuntime>::Queue<M>;

/// Helper alias mapping an actor runtime to the signal type of its mailbox runtime.
pub type MailboxSignalOf<R> = <MailboxOf<R> as MailboxRuntime>::Signal;

/// Helper alias mapping an actor runtime to the concurrency marker of its mailbox runtime.
pub type MailboxConcurrencyOf<R> = <MailboxOf<R> as MailboxRuntime>::Concurrency;

/// High-level runtime interface that extends [`MailboxRuntime`] with bundle-specific capabilities.
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
  type Mailbox: MailboxRuntime + Clone + 'static;

  /// Returns a shared reference to the underlying mailbox runtime.
  fn mailbox_runtime(&self) -> &Self::Mailbox;

  /// Consumes `self` and returns the underlying mailbox runtime.
  fn into_mailbox_runtime(self) -> Self::Mailbox
  where
    Self: Sized;

  /// Returns the shared handle to the underlying mailbox runtime.
  fn mailbox_runtime_shared(&self) -> ArcShared<Self::Mailbox>;

  /// Returns the receive-timeout factory configured for this runtime.
  fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self>>;

  /// Returns the mailbox-level receive-timeout factory if available.
  fn mailbox_receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self::Mailbox>> {
    None
  }

  /// Returns the receive-timeout driver configured for this runtime.
  fn receive_timeout_driver(&self) -> Option<ReceiveTimeoutDriverShared<Self::Mailbox>>;

  /// Overrides the receive-timeout factory using the base mailbox runtime type.
  fn with_receive_timeout_factory(self, factory: ReceiveTimeoutFactoryShared<DynMessage, Self::Mailbox>) -> Self
  where
    Self: Sized;

  /// Overrides the receive-timeout factory using a runtime-specific factory.
  fn with_receive_timeout_factory_shared(self, factory: ReceiveTimeoutFactoryShared<DynMessage, Self>) -> Self
  where
    Self: Sized;

  /// Overrides the receive-timeout driver.
  fn with_receive_timeout_driver(self, driver: Option<ReceiveTimeoutDriverShared<Self::Mailbox>>) -> Self
  where
    Self: Sized;

  /// Mutably overrides the receive-timeout driver.
  fn set_receive_timeout_driver(&mut self, driver: Option<ReceiveTimeoutDriverShared<Self::Mailbox>>);

  /// Returns a factory constructed from the configured receive-timeout driver, if any.
  fn receive_timeout_driver_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self>>;

  /// Returns the root failure event listener configured for the runtime.
  fn root_event_listener(&self) -> Option<FailureEventListener>;

  /// Overrides the root failure event listener.
  fn with_root_event_listener(self, listener: Option<FailureEventListener>) -> Self
  where
    Self: Sized;

  /// Returns the root escalation handler configured for the runtime.
  fn root_escalation_handler(&self) -> Option<FailureEventHandler>;

  /// Overrides the root escalation handler.
  fn with_root_escalation_handler(self, handler: Option<FailureEventHandler>) -> Self
  where
    Self: Sized;

  /// Returns the metrics sink configured for the runtime.
  fn metrics_sink(&self) -> Option<MetricsSinkShared>;

  /// Overrides the metrics sink.
  fn with_metrics_sink(self, sink: Option<MetricsSinkShared>) -> Self
  where
    Self: Sized;

  /// Overrides the metrics sink using a shared handle.
  fn with_metrics_sink_shared(self, sink: MetricsSinkShared) -> Self
  where
    Self: Sized;

  /// Returns a priority mailbox spawner handle without exposing the internal factory.
  fn priority_mailbox_spawner<M>(&self) -> PriorityMailboxSpawnerHandle<M, Self::Mailbox>
  where
    M: Element,
    MailboxQueueOf<Self, PriorityEnvelope<M>>: Clone,
    MailboxSignalOf<Self>: Clone;

  /// Overrides the scheduler builder used during actor system construction.
  fn with_scheduler_builder(self, builder: SchedulerBuilder<DynMessage, Self::Mailbox>) -> Self
  where
    Self: Sized;

  /// Overrides the scheduler builder using a shared handle.
  fn with_scheduler_builder_shared(self, builder: ArcShared<SchedulerBuilder<DynMessage, Self::Mailbox>>) -> Self
  where
    Self: Sized;

  /// Returns the scheduler builder configured for this runtime.
  fn scheduler_builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, Self::Mailbox>>;
}
