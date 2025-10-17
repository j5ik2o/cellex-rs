use super::{Behavior, BehaviorDirective, Signal, SignalFn, SupervisorStrategyConfig, SystemHandlerFn};
use crate::api::actor::context::Context;
use crate::api::actor::failure::ActorFailure;
use crate::api::actor_runtime::{ActorRuntime, MailboxConcurrencyOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::mailbox::messages::PriorityEnvelope;
use crate::api::mailbox::messages::SystemMessage;
use crate::api::messaging::MessageEnvelope;
use crate::{DynMessage, MapSystemShared, MetadataStorageMode};
use alloc::boxed::Box;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;
use core::mem;

/// Adapter that bridges Behavior to untyped runtime.
pub struct ActorAdapter<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone, {
  behavior_factory: ArcShared<dyn Fn() -> Behavior<U, R> + 'static>,
  pub(super) behavior: Behavior<U, R>,
  pub(super) system_handler: Option<Box<SystemHandlerFn<U, R>>>,
}

impl<U, R> ActorAdapter<U, R>
where
  U: Element,
  R: ActorRuntime + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  MailboxConcurrencyOf<R>: MetadataStorageMode,
{
  /// Creates a new `ActorAdapter`.
  pub fn new<S>(behavior_factory: ArcShared<dyn Fn() -> Behavior<U, R> + 'static>, system_handler: Option<S>) -> Self
  where
    S: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, SystemMessage) + 'static, {
    let behavior = behavior_factory();
    Self {
      behavior_factory,
      behavior,
      system_handler: system_handler.map(|h| Box::new(h) as Box<SystemHandlerFn<U, R>>),
    }
  }

  /// Processes a user message.
  pub fn handle_user(&mut self, ctx: &mut Context<'_, '_, U, R>, message: U) -> Result<(), ActorFailure> {
    self.ensure_initialized(ctx)?;
    while matches!(self.behavior, Behavior::Setup { .. }) {
      self.ensure_initialized(ctx)?;
    }
    match &mut self.behavior {
      Behavior::Receive(state) => match state.handle(ctx, message)? {
        BehaviorDirective::Same => {}
        BehaviorDirective::Become(next) => self.transition(next, ctx)?,
      },
      Behavior::Stopped => {}
      Behavior::Setup { .. } => {
        return Err(ActorFailure::from_message("behavior remained in setup state"));
      }
    }
    Ok(())
  }

  /// Processes a system message.
  pub fn handle_system(&mut self, ctx: &mut Context<'_, '_, U, R>, message: SystemMessage) -> Result<(), ActorFailure> {
    self.ensure_initialized(ctx)?;
    while matches!(self.behavior, Behavior::Setup { .. }) {
      self.ensure_initialized(ctx)?;
    }
    if matches!(message, SystemMessage::Stop) {
      self.transition(Behavior::stopped(), ctx)?;
    } else if matches!(message, SystemMessage::Restart) {
      self.behavior = (self.behavior_factory)();
      self.ensure_initialized(ctx)?;
    }
    if matches!(self.behavior, Behavior::Setup { .. }) {
      return Err(ActorFailure::from_message("behavior remained in setup state"));
    }
    if let Some(handler) = self.system_handler.as_mut() {
      handler(ctx, message);
    }
    Ok(())
  }

  /// Creates a SystemMessage mapper for Guardian/Scheduler.
  #[must_use]
  pub fn create_map_system() -> MapSystemShared<DynMessage> {
    MapSystemShared::new(|sys| DynMessage::new(MessageEnvelope::<U>::System(sys)))
  }

  /// Gets supervisor configuration (internal API).
  pub(crate) fn supervisor_config(&self) -> SupervisorStrategyConfig {
    self.behavior.supervisor_config()
  }

  fn ensure_initialized(&mut self, ctx: &mut Context<'_, '_, U, R>) -> Result<(), ActorFailure> {
    while matches!(self.behavior, Behavior::Setup { .. }) {
      let (init, signal) = match mem::replace(&mut self.behavior, Behavior::stopped()) {
        Behavior::Setup { init, signal } => (init, signal),
        other => {
          self.behavior = other;
          break;
        }
      };
      let next_signal = signal.clone();
      let next_behavior = if let Some(init) = init {
        init(ctx)?
      } else {
        Behavior::stopped()
      };
      self.behavior = next_behavior.attach_signal_arc(next_signal);
    }
    Ok(())
  }

  fn current_signal_handler(&self) -> Option<ArcShared<SignalFn<U, R>>> {
    match &self.behavior {
      Behavior::Receive(state) => state.signal_handler(),
      Behavior::Setup { signal, .. } => signal.clone(),
      Behavior::Stopped => None,
    }
  }

  #[allow(dead_code)]
  fn handle_signal(&mut self, ctx: &mut Context<'_, '_, U, R>, signal: Signal) -> Result<(), ActorFailure> {
    if let Some(handler) = self.current_signal_handler() {
      match handler(ctx, signal) {
        BehaviorDirective::Same => {}
        BehaviorDirective::Become(next) => self.transition(next, ctx)?,
      }
    }
    Ok(())
  }

  fn transition(&mut self, next: Behavior<U, R>, ctx: &mut Context<'_, '_, U, R>) -> Result<(), ActorFailure> {
    let previous_handler = self.current_signal_handler();
    self.behavior = next;
    self.ensure_initialized(ctx)?;
    if matches!(self.behavior, Behavior::Stopped) {
      let mut handler = self.current_signal_handler();
      if handler.is_none() {
        handler = previous_handler;
      }
      if let Some(handler) = handler {
        match handler(ctx, Signal::PostStop) {
          BehaviorDirective::Same => {}
          BehaviorDirective::Become(next) => self.transition(next, ctx)?,
        }
      }
    }
    Ok(())
  }
}
