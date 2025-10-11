use core::convert::Infallible;

use crate::runtime::guardian::{AlwaysRestart, GuardianStrategy};
use crate::runtime::scheduler::{SchedulerBuilder, SchedulerHandle};
use crate::ReceiveTimeoutFactoryShared;
use crate::{Extensions, FailureEventHandler, FailureEventListener, MailboxFactory, PriorityEnvelope};
use cellex_utils_core_rs::sync::{ArcShared, Shared};
use cellex_utils_core_rs::{Element, QueueError};
use core::marker::PhantomData;
#[cfg(feature = "std")]
use futures::executor::block_on;

use super::InternalRootContext;

/// Internal configuration used while assembling [`InternalActorSystem`].
pub struct InternalActorSystemSettings<M, R>
where
  M: Element,
  R: MailboxFactory + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// Listener invoked for failures reaching the root guardian.
  pub(crate) root_event_listener: Option<FailureEventListener>,
  /// Escalation handler invoked when failures bubble to the root guardian.
  pub(crate) root_escalation_handler: Option<FailureEventHandler>,
  /// Receive-timeout scheduler factory applied to newly spawned actors.
  pub(crate) receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<M, R>>,
  /// Shared registry of actor system extensions.
  pub(crate) extensions: Extensions,
}

impl<M, R> Default for InternalActorSystemSettings<M, R>
where
  M: Element,
  R: MailboxFactory + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  fn default() -> Self {
    Self {
      root_event_listener: None,
      root_escalation_handler: None,
      receive_timeout_factory: None,
      extensions: Extensions::new(),
    }
  }
}

pub(crate) struct InternalActorSystem<M, R, Strat = AlwaysRestart>
where
  M: Element + 'static,
  R: MailboxFactory + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>, {
  pub(super) scheduler: SchedulerHandle<M, R>,
  pub(super) runtime: ArcShared<R>,
  extensions: Extensions,
  _strategy: PhantomData<Strat>,
}

#[allow(dead_code)]
impl<M, R> InternalActorSystem<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxFactory + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
{
  pub fn new(mailbox_factory: R) -> Self {
    Self::new_with_settings(mailbox_factory, InternalActorSystemSettings::default())
  }

  pub fn new_with_settings(mailbox_factory: R, settings: InternalActorSystemSettings<M, R>) -> Self {
    let scheduler_builder = ArcShared::new(SchedulerBuilder::<M, R>::priority());
    Self::new_with_settings_and_builder(mailbox_factory, scheduler_builder, settings)
  }

  pub fn new_with_settings_and_builder(
    mailbox_factory: R,
    scheduler_builder: ArcShared<SchedulerBuilder<M, R>>,
    settings: InternalActorSystemSettings<M, R>,
  ) -> Self {
    let InternalActorSystemSettings {
      root_event_listener,
      root_escalation_handler,
      receive_timeout_factory,
      extensions,
    } = settings;
    let factory_shared = ArcShared::new(mailbox_factory);
    let runtime = factory_shared.clone();
    let factory_for_scheduler = factory_shared.with_ref(|factory| factory.clone());
    let mut scheduler = scheduler_builder.build(factory_for_scheduler, extensions.clone());
    scheduler.set_root_event_listener(root_event_listener);
    scheduler.set_root_escalation_handler(root_escalation_handler);
    scheduler.set_receive_timeout_factory(receive_timeout_factory);
    Self {
      scheduler,
      runtime,
      extensions,
      _strategy: PhantomData,
    }
  }
}

impl<M, R, Strat> InternalActorSystem<M, R, Strat>
where
  M: Element,
  R: MailboxFactory + Clone,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Strat: GuardianStrategy<M, R>,
{
  pub fn root_context(&mut self) -> InternalRootContext<'_, M, R, Strat> {
    InternalRootContext { system: self }
  }

  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    self.run_until_impl(should_continue).await
  }

  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    loop {
      self.scheduler.dispatch_next().await?;
    }
  }

  #[cfg(feature = "std")]
  pub fn blocking_dispatch_loop<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    self.blocking_dispatch_loop_impl(should_continue)
  }

  #[cfg(feature = "std")]
  pub fn blocking_dispatch_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    loop {
      block_on(self.scheduler.dispatch_next())?;
    }
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.scheduler.dispatch_next().await
  }

  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.scheduler.drain_ready()
  }

  pub fn run_until_idle<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      let processed = self.drain_ready()?;
      if !processed {
        break;
      }
    }
    Ok(())
  }

  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  async fn run_until_impl<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.scheduler.dispatch_next().await?;
    }
    Ok(())
  }

  #[cfg(feature = "std")]
  fn blocking_dispatch_loop_impl<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      block_on(self.scheduler.dispatch_next())?;
    }
    Ok(())
  }
}
