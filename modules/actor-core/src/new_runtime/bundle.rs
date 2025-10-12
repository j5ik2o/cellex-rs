//! Defines the bundle abstraction consumed by the new actor system.

use alloc::sync::Arc;

use cellex_utils_core_rs::sync::ArcShared;

use crate::runtime::mailbox::traits::MailboxRuntime;
use crate::runtime::message::DynMessage;
use crate::shared::ReceiveTimeoutFactoryShared;
use crate::{Extensions, FailureEventHandler, FailureEventListener, MetricsSinkShared, PriorityEnvelope};

use super::mailbox::{NewMailboxHandleFactory, NewMailboxRuntime};
use super::runtime_parts::RuntimeParts;
use super::scheduler::NewSchedulerBuilder;

/// Bundle abstraction exposed by the new actor runtime API.
pub trait NewActorRuntimeBundle: Clone + Send + Sync + 'static {
  /// Mailbox runtime type used by the internal actor system.
  type MailboxRuntime: NewMailboxRuntime + Clone + 'static;
  /// Scheduler builder implementation bound to the mailbox runtime.
  type SchedulerBuilder: NewSchedulerBuilder<Self::MailboxRuntime> + 'static;

  /// Returns the mailbox handle factory shared across the system.
  fn mailbox_handle_factory(&self) -> ArcShared<dyn NewMailboxHandleFactory<Self::MailboxRuntime>>;

  /// Returns the scheduler builder associated with this bundle.
  fn scheduler_builder(&self) -> ArcShared<Self::SchedulerBuilder>;

  /// Returns an optional receive-timeout factory.
  fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self::MailboxRuntime>>;

  /// Returns the metrics sink applied to runtime components.
  fn metrics_sink(&self) -> Option<MetricsSinkShared>;

  /// Returns the root failure event listener.
  fn root_event_listener(&self) -> Option<FailureEventListener>;

  /// Returns the root escalation handler.
  fn root_escalation_handler(&self) -> Option<FailureEventHandler>;

  /// Returns the extension registry view.
  fn extensions(&self) -> &Extensions;

  /// Builds the [`RuntimeParts`] view consumed by the new actor system.
  fn runtime_parts(&self) -> RuntimeParts<Self::MailboxRuntime>
  where
    Self::MailboxRuntime: Clone,
    <Self::MailboxRuntime as MailboxRuntime>::Queue<PriorityEnvelope<DynMessage>>: Clone,
    <Self::MailboxRuntime as MailboxRuntime>::Signal: Clone,
    <Self::MailboxRuntime as MailboxRuntime>::Producer<PriorityEnvelope<DynMessage>>: Clone, {
    let mailbox_factory = self.mailbox_handle_factory();
    let scheduler_builder_impl = self.scheduler_builder();
    let scheduler_builder_arc: Arc<Self::SchedulerBuilder> = ArcShared::into_arc(scheduler_builder_impl);
    let scheduler_builder_trait: Arc<dyn NewSchedulerBuilder<Self::MailboxRuntime>> = scheduler_builder_arc;
    let scheduler_builder = ArcShared::from_arc(scheduler_builder_trait);
    let extensions = self.extensions().clone();
    RuntimeParts::new(mailbox_factory, scheduler_builder, extensions)
      .with_receive_timeout_factory(self.receive_timeout_factory())
      .with_metrics_sink(self.metrics_sink())
      .with_failure_handlers(self.root_event_listener(), self.root_escalation_handler())
  }
}
