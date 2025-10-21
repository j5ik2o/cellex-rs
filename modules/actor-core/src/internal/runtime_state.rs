use cellex_utils_core_rs::sync::{ArcShared, Shared};

use crate::{
  api::{actor_scheduler::ActorSchedulerHandleBuilder, mailbox::MailboxFactory},
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

/// Internal state container for `GenericActorRuntime`.
///
/// This structure holds the use cellex_actor_core_rs::api::mailbox::MailboxRuntime; and scheduler
/// builder configuration used by the actor runtime implementation.
#[derive(Clone)]
pub(crate) struct GenericActorRuntimeState<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  mailbox_factory:   ArcShared<MF>,
  scheduler_builder: ArcShared<ActorSchedulerHandleBuilder<MF>>,
}

impl<MF> GenericActorRuntimeState<MF>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
{
  /// Creates a new runtime state with the given use
  /// cellex_actor_core_rs::api::mailbox::MailboxRuntime;.
  ///
  /// Initializes with a default ready-queue scheduler builder.
  #[must_use]
  pub(crate) fn new(mailbox_factory: MF) -> Self {
    Self {
      mailbox_factory:   ArcShared::new(mailbox_factory),
      scheduler_builder: ArcShared::new(ActorSchedulerHandleBuilder::<MF>::ready_queue()),
    }
  }

  /// Returns a reference to the use cellex_actor_core_rs::api::mailbox::MailboxRuntime;.
  #[must_use]
  #[allow(clippy::missing_const_for_fn)]
  pub(crate) fn mailbox_factory(&self) -> &MF {
    &self.mailbox_factory
  }

  /// Returns a shared handle to the [`MailboxFactory`].
  #[must_use]
  pub(crate) fn mailbox_factory_shared(&self) -> ArcShared<MF> {
    self.mailbox_factory.clone()
  }

  /// Consumes this state and returns the contained [`MailboxFactory`].
  ///
  /// If the shared handle has other references, clones the runtime.
  #[must_use]
  pub(crate) fn into_mailbox_factory(self) -> MF {
    self.mailbox_factory.try_unwrap().unwrap_or_else(|shared| (*shared).clone())
  }

  /// Returns a shared handle to the scheduler builder.
  #[must_use]
  pub(crate) fn scheduler_builder(&self) -> ArcShared<ActorSchedulerHandleBuilder<MF>> {
    self.scheduler_builder.clone()
  }

  /// Sets the scheduler builder for this runtime state.
  pub(crate) fn set_scheduler_builder(&mut self, builder: ArcShared<ActorSchedulerHandleBuilder<MF>>) {
    self.scheduler_builder = builder;
  }
}
