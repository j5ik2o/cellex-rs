//! Shared runtime components required to bootstrap the new actor system.

use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;

use crate::runtime::message::DynMessage;
use crate::shared::ReceiveTimeoutFactoryShared;
use crate::{Extensions, FailureEventHandler, FailureEventListener, MetricsSinkShared, PriorityEnvelope};

use super::mailbox::{NewMailboxHandleFactory, NewMailboxRuntime};
use super::scheduler::NewSchedulerBuilder;

/// Shared collection of runtime components required to bootstrap the new actor system.
#[derive(Clone)]
pub struct RuntimeParts<R, M = DynMessage>
where
  R: NewMailboxRuntime + Clone + 'static,
  M: Element + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone, {
  /// Shared mailbox factory keeping ownership of the runtime instance.
  pub mailbox_factory: ArcShared<dyn NewMailboxHandleFactory<R>>,
  /// Scheduler builder handle bound to the runtime type.
  pub scheduler_builder: ArcShared<dyn NewSchedulerBuilder<R>>,
  /// Optional receive-timeout factory propagated to internal settings.
  pub receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<M, R>>,
  /// Metrics sink applied to the runtime where supported.
  pub metrics_sink: Option<MetricsSinkShared>,
  /// Listener invoked when failures reach the root guardian.
  pub root_event_listener: Option<FailureEventListener>,
  /// Handler invoked when failures escalate from the root guardian.
  pub root_escalation_handler: Option<FailureEventHandler>,
  /// Extension registry snapshot shared with the actor system.
  pub extensions: Extensions,
}

impl<R, M> RuntimeParts<R, M>
where
  R: NewMailboxRuntime + Clone + 'static,
  M: Element + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  /// Creates a new [`RuntimeParts`] instance with mandatory components.
  #[must_use]
  pub fn new(
    mailbox_factory: ArcShared<dyn NewMailboxHandleFactory<R>>,
    scheduler_builder: ArcShared<dyn NewSchedulerBuilder<R>>,
    extensions: Extensions,
  ) -> Self {
    Self {
      mailbox_factory,
      scheduler_builder,
      receive_timeout_factory: None,
      metrics_sink: None,
      root_event_listener: None,
      root_escalation_handler: None,
      extensions,
    }
  }

  /// Registers a prebuilt receive-timeout factory.
  #[must_use]
  pub fn with_receive_timeout_factory(mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) -> Self {
    self.receive_timeout_factory = factory;
    self
  }

  /// Applies the shared metrics sink for the runtime.
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Sets the root failure handlers derived from bundle configuration.
  #[must_use]
  pub fn with_failure_handlers(
    mut self,
    listener: Option<FailureEventListener>,
    handler: Option<FailureEventHandler>,
  ) -> Self {
    self.root_event_listener = listener;
    self.root_escalation_handler = handler;
    self
  }

  /// Provides the receive-timeout factory if one has been configured.
  #[must_use]
  pub fn resolve_receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<M, R>> {
    self.receive_timeout_factory.clone()
  }
}
