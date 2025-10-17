use alloc::boxed::Box;

use crate::api::mailbox::MailboxFactory;
use crate::api::mailbox::PriorityEnvelope;
use crate::internal::scheduler::SchedulerFactoryBound;
use crate::shared::map_system::MapSystemShared;
use crate::shared::receive_timeout::ReceiveTimeoutScheduler;
use cellex_utils_core_rs::Element;

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
  R: MailboxFactory + Clone + 'static,
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
