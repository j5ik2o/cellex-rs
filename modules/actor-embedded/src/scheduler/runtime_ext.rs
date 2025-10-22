#![cfg(feature = "embassy_executor")]

use alloc::boxed::Box;

use cellex_actor_core_rs::{
  api::{
    actor_runtime::GenericActorRuntime, actor_scheduler::ActorSchedulerHandleBuilder, guardian::AlwaysRestart,
    mailbox::MailboxFactory, receive_timeout::ReceiveTimeoutSchedulerFactoryShared,
  },
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};
use embassy_executor::Spawner;

use super::embassy_scheduler_impl::EmbassyScheduler;
use crate::receive_timeout::EmbassyReceiveTimeoutSchedulerFactory;

/// Utility that produces an Embassy-ready scheduler builder.
#[must_use]
pub fn embassy_scheduler_builder<MF>() -> ActorSchedulerHandleBuilder<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  ActorSchedulerHandleBuilder::new(|mailbox_factory, extensions| {
    Box::new(EmbassyScheduler::<MF, AlwaysRestart>::new(mailbox_factory, extensions))
  })
}

/// Extension trait that installs the Embassy scheduler into a [`GenericActorRuntime`].
pub trait EmbassyActorRuntimeExt<MF>
where
  MF: MailboxFactory + Clone + Send + Sync + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  /// Replaces the scheduler with the Embassy-backed implementation.
  fn with_embassy_scheduler(self, spawner: &'static Spawner) -> GenericActorRuntime<MF>;
}

impl<MF> EmbassyActorRuntimeExt<MF> for GenericActorRuntime<MF>
where
  MF: MailboxFactory + Clone + Send + Sync + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  fn with_embassy_scheduler(self, spawner: &'static Spawner) -> GenericActorRuntime<MF> {
    let bundle = self.with_scheduler_builder(embassy_scheduler_builder());
    if bundle.receive_timeout_scheduler_factory_shared().is_some() {
      bundle
    } else {
      bundle.with_receive_timeout_scheduler_factory_shared(ReceiveTimeoutSchedulerFactoryShared::new(
        EmbassyReceiveTimeoutSchedulerFactory::<MF>::new(spawner),
      ))
    }
  }
}
