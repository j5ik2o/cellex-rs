use alloc::boxed::Box;
use core::time::Duration;

use cellex_utils_core_rs::Element;

use crate::runtime::message::DynMessage;
use crate::shared::{ReceiveTimeoutDriver, ReceiveTimeoutFactoryShared};
use crate::MapSystemShared;
use crate::{MailboxRuntime, PriorityEnvelope};

#[cfg(target_has_atomic = "ptr")]
pub trait SchedulerBound: Send {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send> SchedulerBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub trait SchedulerBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SchedulerBound for T {}

#[cfg(target_has_atomic = "ptr")]
pub trait SchedulerFactoryBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> SchedulerFactoryBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub trait SchedulerFactoryBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> SchedulerFactoryBound for T {}

/// Scheduler abstraction for managing actor `ReceiveTimeout`.
///
/// Provides a unified interface for setting/resetting/stopping timeouts,
/// so that `actor-core` doesn't need to directly handle runtime-dependent timers.
/// By calling `notify_activity` after user message processing,
/// the runtime side can re-arm with any implementation (tokio / embedded software timer, etc.).
pub trait ReceiveTimeoutScheduler: SchedulerBound {
  /// Sets/re-arms the timer with the specified duration.
  fn set(&mut self, duration: Duration);

  /// Stops the timer.
  fn cancel(&mut self);

  /// Notifies of activity (like user messages) that should reset the timeout.
  fn notify_activity(&mut self);
}

/// Factory for creating schedulers.
///
/// Receives a priority mailbox and SystemMessage conversion function when creating actors,
/// and assembles a runtime-specific `ReceiveTimeoutScheduler`.
/// By configuring the system through `ActorSystemConfig::with_receive_timeout_factory` or
/// `ActorSystemConfig::set_receive_timeout_factory` before constructing it,
/// all actors can handle timeouts with the same policy.
pub trait ReceiveTimeoutSchedulerFactory<M, R>: SchedulerFactoryBound
where
  M: Element + 'static,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone, {
  /// Creates an actor-specific scheduler by receiving a priority mailbox and SystemMessage conversion function.
  fn create(
    &self,
    sender: R::Producer<PriorityEnvelope<M>>,
    map_system: MapSystemShared<M>,
  ) -> Box<dyn ReceiveTimeoutScheduler>;
}

/// `ReceiveTimeoutScheduler` implementation that performs no scheduling.
#[derive(Default)]
pub struct NoopReceiveTimeoutScheduler;

impl ReceiveTimeoutScheduler for NoopReceiveTimeoutScheduler {
  fn set(&mut self, _duration: core::time::Duration) {}

  fn cancel(&mut self) {}

  fn notify_activity(&mut self) {}
}

/// Factory that returns [`NoopReceiveTimeoutScheduler`].
#[derive(Debug, Default, Clone)]
pub struct NoopReceiveTimeoutSchedulerFactory;

impl<M, R> ReceiveTimeoutSchedulerFactory<M, R> for NoopReceiveTimeoutSchedulerFactory
where
  M: Element + 'static,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<M>>: Clone,
{
  fn create(
    &self,
    _sender: R::Producer<PriorityEnvelope<M>>,
    _map_system: MapSystemShared<M>,
  ) -> Box<dyn ReceiveTimeoutScheduler> {
    Box::new(NoopReceiveTimeoutScheduler)
  }
}

/// Driver that always provides [`NoopReceiveTimeoutSchedulerFactory`].
#[derive(Debug, Default, Clone)]
pub struct NoopReceiveTimeoutDriver;

impl<R> ReceiveTimeoutDriver<R> for NoopReceiveTimeoutDriver
where
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Producer<PriorityEnvelope<DynMessage>>: Clone,
{
  fn build_factory(&self) -> ReceiveTimeoutFactoryShared<DynMessage, R> {
    ReceiveTimeoutFactoryShared::new(NoopReceiveTimeoutSchedulerFactory)
  }
}
