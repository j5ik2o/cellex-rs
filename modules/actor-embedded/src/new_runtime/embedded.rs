#![cfg(feature = "new-runtime")]

//! `NewActorRuntimeBundle` implementation for embedded environments.

use alloc::sync::Arc;

use cellex_actor_core_rs::{DynMessage, Extensions, FailureEventHandler, FailureEventListener, MailboxHandleFactoryStub, MetricsSinkShared, NewActorRuntimeBundle, NewMailboxHandleFactory, NoopReceiveTimeoutSchedulerFactory, ReceiveTimeoutFactoryShared, SchedulerBuilder, SharedSchedulerBuilder};
use cellex_utils_core_rs::sync::ArcShared;

use crate::local_mailbox::LocalMailboxRuntime;

/// Lightweight bundle tailored for `LocalMailboxRuntime`.
#[derive(Clone)]
pub struct EmbeddedBundle {
  mailbox_factory: ArcShared<MailboxHandleFactoryStub<LocalMailboxRuntime>>,
  scheduler_builder: ArcShared<SharedSchedulerBuilder<LocalMailboxRuntime>>,
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<DynMessage, LocalMailboxRuntime>>,
  metrics_sink: Option<MetricsSinkShared>,
  root_event_listener: Option<FailureEventListener>,
  root_escalation_handler: Option<FailureEventHandler>,
  extensions: Extensions,
}

impl EmbeddedBundle {
  /// Creates a bundle using `LocalMailboxRuntime` and a priority scheduler.
  #[must_use]
  pub fn new() -> Self {
    let runtime = LocalMailboxRuntime::default();
    let mailbox_stub = MailboxHandleFactoryStub::from_runtime(runtime);
    let mailbox_factory = ArcShared::new(mailbox_stub);

    let scheduler = SharedSchedulerBuilder::from_builder(SchedulerBuilder::<DynMessage, LocalMailboxRuntime>::priority());
    let scheduler_builder = ArcShared::new(scheduler);

    let receive_timeout_factory = Some(ReceiveTimeoutFactoryShared::new(NoopReceiveTimeoutSchedulerFactory::default()));

    Self {
      mailbox_factory,
      scheduler_builder,
      receive_timeout_factory,
      metrics_sink: None,
      root_event_listener: None,
      root_escalation_handler: None,
      extensions: Extensions::new(),
    }
  }

  /// Overrides the metrics sink applied to spawned actors.
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Overrides the root failure event listener.
  #[must_use]
  pub fn with_root_event_listener(mut self, listener: Option<FailureEventListener>) -> Self {
    self.root_event_listener = listener;
    self
  }

  /// Overrides the root escalation handler.
  #[must_use]
  pub fn with_root_escalation_handler(mut self, handler: Option<FailureEventHandler>) -> Self {
    self.root_escalation_handler = handler;
    self
  }

  /// Overrides the receive-timeout factory.
  #[must_use]
  pub fn with_receive_timeout_factory(
    mut self,
    factory: Option<ReceiveTimeoutFactoryShared<DynMessage, LocalMailboxRuntime>>,
  ) -> Self {
    self.receive_timeout_factory = factory;
    self
  }

  /// Provides mutable access to the extension registry.
  pub fn extensions_mut(&mut self) -> &mut Extensions {
    &mut self.extensions
  }
}

impl Default for EmbeddedBundle {
  fn default() -> Self {
    Self::new()
  }
}

impl NewActorRuntimeBundle for EmbeddedBundle {
  type MailboxRuntime = LocalMailboxRuntime;
  type SchedulerBuilder = SharedSchedulerBuilder<LocalMailboxRuntime>;

  fn mailbox_handle_factory(&self) -> ArcShared<dyn NewMailboxHandleFactory<Self::MailboxRuntime>> {
    let stub = self.mailbox_factory.clone();
    let arc: Arc<MailboxHandleFactoryStub<Self::MailboxRuntime>> = ArcShared::into_arc(stub);
    ArcShared::from_arc(arc as Arc<dyn NewMailboxHandleFactory<Self::MailboxRuntime>>)
  }

  fn scheduler_builder(&self) -> ArcShared<Self::SchedulerBuilder> {
    self.scheduler_builder.clone()
  }

  fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self::MailboxRuntime>> {
    self.receive_timeout_factory.clone()
  }

  fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink.clone()
  }

  fn root_event_listener(&self) -> Option<FailureEventListener> {
    self.root_event_listener.clone()
  }

  fn root_escalation_handler(&self) -> Option<FailureEventHandler> {
    self.root_escalation_handler.clone()
  }

  fn extensions(&self) -> &Extensions {
    &self.extensions
  }
}
