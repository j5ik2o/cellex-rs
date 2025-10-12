//! Scheduler builder helpers reused by the new runtime layer.

use cellex_utils_core_rs::sync::ArcShared;

use crate::runtime::message::DynMessage;
use crate::runtime::scheduler::SchedulerBuilder;
use crate::PriorityEnvelope;

use super::mailbox::NewMailboxRuntime;

/// Abstracts scheduler builder access for the new runtime API.
pub trait NewSchedulerBuilder<R>: Send + Sync
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  /// Returns the shared handle to the underlying [`SchedulerBuilder`].
  fn builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, R>>;
}

/// Default wrapper that stores a shared [`SchedulerBuilder`].
#[derive(Clone)]
pub struct SharedSchedulerBuilder<R>
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  builder: ArcShared<SchedulerBuilder<DynMessage, R>>,
}

impl<R> SharedSchedulerBuilder<R>
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  /// Creates the wrapper from a shared builder handle.
  #[must_use]
  pub fn new(builder: ArcShared<SchedulerBuilder<DynMessage, R>>) -> Self {
    Self { builder }
  }

  /// Wraps an owned builder by cloning it into an [`ArcShared`].
  #[must_use]
  pub fn from_builder(builder: SchedulerBuilder<DynMessage, R>) -> Self {
    Self {
      builder: ArcShared::new(builder),
    }
  }
}

impl<R> NewSchedulerBuilder<R> for SharedSchedulerBuilder<R>
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn builder(&self) -> ArcShared<SchedulerBuilder<DynMessage, R>> {
    self.builder.clone()
  }
}

impl<R> From<ArcShared<SchedulerBuilder<DynMessage, R>>> for SharedSchedulerBuilder<R>
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn from(builder: ArcShared<SchedulerBuilder<DynMessage, R>>) -> Self {
    Self::new(builder)
  }
}

impl<R> From<SchedulerBuilder<DynMessage, R>> for SharedSchedulerBuilder<R>
where
  R: NewMailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
{
  fn from(builder: SchedulerBuilder<DynMessage, R>) -> Self {
    Self::from_builder(builder)
  }
}
