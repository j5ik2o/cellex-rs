//! Adapter traits for mailbox runtimes exposed by the new API.

use cellex_utils_core_rs::sync::ArcShared;

use crate::api::actor::MailboxHandleFactoryStub;
use crate::runtime::mailbox::traits::MailboxRuntime;
use crate::runtime::mailbox::PriorityMailboxSpawnerHandle;
use crate::runtime::message::DynMessage;
use crate::runtime::metrics::MetricsSinkShared;
use crate::PriorityEnvelope;

/// Marker trait used by the new runtime API to describe mailbox runtimes.
pub trait NewMailboxRuntime: MailboxRuntime {}

impl<T> NewMailboxRuntime for T where T: MailboxRuntime {}

/// Shared factory interface re-exported for the new API surface.
pub trait NewMailboxHandleFactory<R>
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  /// Returns the shared runtime handle used to spawn mailboxes.
  fn runtime_shared(&self) -> ArcShared<R>;

  /// Returns a priority mailbox spawner configured for dynamic messages.
  fn priority_spawner(&self) -> PriorityMailboxSpawnerHandle<DynMessage, R>;

  /// Returns the metrics sink applied to spawned mailboxes.
  fn metrics_sink(&self) -> Option<MetricsSinkShared>;
}

impl<R> NewMailboxHandleFactory<R> for MailboxHandleFactoryStub<R>
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn runtime_shared(&self) -> ArcShared<R> {
    self.runtime_shared()
  }

  fn priority_spawner(&self) -> PriorityMailboxSpawnerHandle<DynMessage, R> {
    self.priority_spawner()
  }

  fn metrics_sink(&self) -> Option<MetricsSinkShared> {
    self.metrics_sink()
  }
}
