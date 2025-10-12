#![cfg(any(test, feature = "test-support"))]

//! Test harness bundle leveraging the existing in-memory mailbox runtime.

use alloc::sync::Arc;

use cellex_utils_core_rs::sync::ArcShared;

use crate::api::actor::MailboxHandleFactoryStub;
use crate::runtime::mailbox::test_support::TestMailboxRuntime;
use crate::runtime::message::DynMessage;
use crate::runtime::scheduler::SchedulerBuilder;
use crate::shared::ReceiveTimeoutFactoryShared;
use crate::{
  Extensions, FailureEventHandler, FailureEventListener, MetricsSinkShared, NoopReceiveTimeoutSchedulerFactory,
};

use super::bundle::NewActorRuntimeBundle;
use super::mailbox::NewMailboxHandleFactory;
use super::scheduler::SharedSchedulerBuilder;

/// Bundle implementation tailored for unit and integration tests.
#[derive(Clone)]
pub struct TestHarnessBundle {
  mailbox_factory: ArcShared<MailboxHandleFactoryStub<TestMailboxRuntime>>,
  scheduler_builder: ArcShared<SharedSchedulerBuilder<TestMailboxRuntime>>,
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<DynMessage, TestMailboxRuntime>>,
  metrics_sink: Option<MetricsSinkShared>,
  root_event_listener: Option<FailureEventListener>,
  root_escalation_handler: Option<FailureEventHandler>,
  extensions: Extensions,
}

impl Default for TestHarnessBundle {
  fn default() -> Self {
    Self::new()
  }
}

impl TestHarnessBundle {
  /// Creates a bundle using the default in-memory mailbox runtime.
  #[must_use]
  pub fn new() -> Self {
    let runtime = TestMailboxRuntime::unbounded();
    let mailbox_stub = MailboxHandleFactoryStub::from_runtime(runtime.clone());
    let mailbox_factory = ArcShared::new(mailbox_stub);

    let scheduler = SharedSchedulerBuilder::from_builder(SchedulerBuilder::<DynMessage, TestMailboxRuntime>::priority());
    let scheduler_builder = ArcShared::new(scheduler);

    let receive_timeout_factory = Some(ReceiveTimeoutFactoryShared::new(
      NoopReceiveTimeoutSchedulerFactory::default(),
    ));

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

  /// Overrides the metrics sink applied to spawned mailboxes.
  #[must_use]
  pub fn with_metrics_sink(mut self, sink: Option<MetricsSinkShared>) -> Self {
    self.metrics_sink = sink;
    self
  }

  /// Overrides the failure event listener applied at the root guardian.
  #[must_use]
  pub fn with_root_event_listener(mut self, listener: Option<FailureEventListener>) -> Self {
    self.root_event_listener = listener;
    self
  }

  /// Overrides the failure escalation handler applied at the root guardian.
  #[must_use]
  pub fn with_root_escalation_handler(mut self, handler: Option<FailureEventHandler>) -> Self {
    self.root_escalation_handler = handler;
    self
  }

  /// Provides mutable access to the extension registry for customisation in tests.
  pub fn extensions_mut(&mut self) -> &mut Extensions {
    &mut self.extensions
  }
}

impl NewActorRuntimeBundle for TestHarnessBundle {
  type MailboxRuntime = TestMailboxRuntime;
  type SchedulerBuilder = SharedSchedulerBuilder<TestMailboxRuntime>;

  fn mailbox_handle_factory(&self) -> ArcShared<dyn NewMailboxHandleFactory<Self::MailboxRuntime>> {
    let cloned = self.mailbox_factory.clone();
    let arc: Arc<MailboxHandleFactoryStub<Self::MailboxRuntime>> = ArcShared::into_arc(cloned);
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
