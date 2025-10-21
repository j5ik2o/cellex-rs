use alloc::boxed::Box;

use cellex_utils_core_rs::{Element, SharedBound};

use crate::api::{
  actor_system::map_system::MapSystemShared,
  mailbox::{messages::PriorityEnvelope, MailboxFactory},
  receive_timeout::ReceiveTimeoutScheduler,
};

/// Factory for creating schedulers.
///
/// Receives a priority mailbox and SystemMessage conversion function when creating actors,
/// and assembles a runtime-specific `ReceiveTimeoutScheduler`.
/// By configuring the system through
/// `GenericActorSystemConfig::with_receive_timeout_scheduler_factory_shared_opt` or
/// `GenericActorSystemConfig::set_receive_timeout_scheduler_factory_shared_opt` before constructing
/// it, all actors can handle timeouts with the same policy.
pub trait ReceiveTimeoutSchedulerFactory<M, MF>: SharedBound
where
  M: Element + 'static,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<M>>: Clone,
  MF::Signal: Clone,
  MF::Producer<PriorityEnvelope<M>>: Clone, {
  /// Creates an actor-specific scheduler by receiving a priority mailbox and SystemMessage
  /// conversion function.
  fn create(
    &self,
    sender: MF::Producer<PriorityEnvelope<M>>,
    map_system: MapSystemShared<M>,
  ) -> Box<dyn ReceiveTimeoutScheduler>;
}
