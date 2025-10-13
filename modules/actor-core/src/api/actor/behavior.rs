use alloc::boxed::Box;
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;

use crate::api::supervision::{NoopSupervisor, Supervisor, SupervisorDirective};
use crate::api::MessageEnvelope;
use crate::runtime::mailbox::traits::ActorRuntime;
use crate::runtime::message::{DynMessage, MetadataStorageMode};
use crate::MapSystemShared;
use crate::PriorityEnvelope;
use crate::SystemMessage;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::Element;
use core::fmt;

use super::{ActorFailure, Context};

type ReceiveFn<U, R> = dyn for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> BehaviorDirective<U, R> + 'static;
type TryReceiveFn<U, R> =
  dyn for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<BehaviorDirective<U, R>, ActorFailure> + 'static;
type SystemHandlerFn<U, R> = dyn for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, SystemMessage) + 'static;
type SignalFn<U, R> = dyn for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>, Signal) -> BehaviorDirective<U, R> + 'static;
type SetupFn<U, R> = dyn for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Behavior<U, R> + 'static;
type TrySetupFn<U, R> =
  dyn for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Result<Behavior<U, R>, ActorFailure> + 'static;

enum ReceiveHandler<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  Simple(Box<ReceiveFn<U, R>>),
  Try(Box<TryReceiveFn<U, R>>),
}

/// Actor lifecycle signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
  /// Signal sent after actor stops
  PostStop,
}

/// Supervisor strategy configuration (internal representation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SupervisorStrategyConfig {
  /// Default strategy (NoopSupervisor)
  Default,
  /// Fixed strategy
  Fixed(SupervisorStrategy),
}

impl SupervisorStrategyConfig {
  pub(crate) fn default() -> Self {
    SupervisorStrategyConfig::Default
  }

  pub(crate) fn from_strategy(strategy: SupervisorStrategy) -> Self {
    SupervisorStrategyConfig::Fixed(strategy)
  }

  pub(crate) fn into_supervisor<M>(&self) -> DynSupervisor<M>
  where
    M: Element, {
    let inner: Box<dyn Supervisor<M>> = match self {
      SupervisorStrategyConfig::Default => Box::new(NoopSupervisor),
      SupervisorStrategyConfig::Fixed(strategy) => Box::new(FixedDirectiveSupervisor::new(*strategy)),
    };
    DynSupervisor::new(inner)
  }
}

/// Types of supervisor strategies.
///
/// Defines how a parent actor handles failures in child actors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorStrategy {
  /// Restart the actor
  Restart,
  /// Stop the actor
  Stop,
  /// Ignore the error and continue processing
  Resume,
  /// Escalate to parent
  Escalate,
}

impl From<SupervisorStrategy> for SupervisorDirective {
  fn from(value: SupervisorStrategy) -> Self {
    match value {
      SupervisorStrategy::Restart => SupervisorDirective::Restart,
      SupervisorStrategy::Stop => SupervisorDirective::Stop,
      SupervisorStrategy::Resume => SupervisorDirective::Resume,
      SupervisorStrategy::Escalate => SupervisorDirective::Escalate,
    }
  }
}

struct FixedDirectiveSupervisor {
  directive: SupervisorDirective,
}

impl FixedDirectiveSupervisor {
  fn new(strategy: SupervisorStrategy) -> Self {
    Self {
      directive: strategy.into(),
    }
  }
}

impl<M> Supervisor<M> for FixedDirectiveSupervisor {
  fn decide(&mut self, _error: &dyn core::fmt::Debug) -> SupervisorDirective {
    self.directive
  }
}

/// Dynamic supervisor implementation (internal type).
pub(crate) struct DynSupervisor<M>
where
  M: Element, {
  inner: Box<dyn Supervisor<M>>,
}

impl<M> DynSupervisor<M>
where
  M: Element,
{
  fn new(inner: Box<dyn Supervisor<M>>) -> Self {
    Self { inner }
  }
}

impl<M> Supervisor<M> for DynSupervisor<M>
where
  M: Element,
{
  fn before_handle(&mut self) {
    self.inner.before_handle();
  }

  fn after_handle(&mut self) {
    self.inner.after_handle();
  }

  fn decide(&mut self, error: &dyn core::fmt::Debug) -> SupervisorDirective {
    self.inner.decide(error)
  }
}

/// State transition directive after user message processing.
///
/// Specifies the next action after message processing.
pub enum BehaviorDirective<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Concurrency: MetadataStorageMode, {
  /// Maintain the same Behavior
  Same,
  /// Transition to a new Behavior
  Become(Behavior<U, R>),
}

/// Struct that holds the internal state of Behavior.
pub struct BehaviorState<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  handler: ReceiveHandler<U, R>,
  supervisor: SupervisorStrategyConfig,
  signal_handler: Option<ArcShared<SignalFn<U, R>>>,
}

impl<U, R> BehaviorState<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Concurrency: MetadataStorageMode,
{
  fn new(handler: ReceiveHandler<U, R>, supervisor: SupervisorStrategyConfig) -> Self {
    Self {
      handler,
      supervisor,
      signal_handler: None,
    }
  }

  fn new_simple(handler: Box<ReceiveFn<U, R>>, supervisor: SupervisorStrategyConfig) -> Self {
    Self::new(ReceiveHandler::Simple(handler), supervisor)
  }

  fn new_try(handler: Box<TryReceiveFn<U, R>>, supervisor: SupervisorStrategyConfig) -> Self {
    Self::new(ReceiveHandler::Try(handler), supervisor)
  }

  fn handle(&mut self, ctx: &mut Context<'_, '_, U, R>, message: U) -> Result<BehaviorDirective<U, R>, ActorFailure> {
    match &mut self.handler {
      ReceiveHandler::Simple(handler) => Ok(handler(ctx, message)),
      ReceiveHandler::Try(handler) => handler(ctx, message),
    }
  }

  fn signal_handler(&self) -> Option<ArcShared<SignalFn<U, R>>> {
    self.signal_handler.clone()
  }

  fn set_signal_handler(&mut self, handler: ArcShared<SignalFn<U, R>>) {
    self.signal_handler = Some(handler);
  }
}

/// Typed Behavior representation. Equivalent to Akka/Pekko Typed's `Behavior`.
///
/// Defines actor behavior. Describes message processing and
/// reactions to lifecycle events.
pub enum Behavior<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  /// Message receiving state
  Receive(BehaviorState<U, R>),
  /// Execute setup processing to generate Behavior
  Setup {
    /// Initialization callback used when setup is infallible.
    init: Option<ArcShared<SetupFn<U, R>>>,
    /// Initialization callback used when setup may fail.
    try_init: Option<ArcShared<TrySetupFn<U, R>>>,
    /// Signal handler retained across behavior transitions.
    signal: Option<ArcShared<SignalFn<U, R>>>,
  },
  /// Stopped state
  Stopped,
}

impl<U, R> Behavior<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Concurrency: MetadataStorageMode,
{
  /// Constructs a `Behavior` with specified message receive handler.
  ///
  /// # Arguments
  /// * `handler` - Processing when message is received
  pub fn receive<F>(handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> BehaviorDirective<U, R> + 'static, {
    Self::Receive(BehaviorState::new_simple(
      Box::new(handler),
      SupervisorStrategyConfig::default(),
    ))
  }

  /// Constructs a `Behavior` whose handler may fail.
  pub fn try_receive<F, E>(mut handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<BehaviorDirective<U, R>, E> + 'static,
    E: fmt::Display + fmt::Debug + Send + 'static, {
    Self::Receive(BehaviorState::new_try(
      Box::new(move |ctx, msg| handler(ctx, msg).map_err(ActorFailure::from_error)),
      SupervisorStrategyConfig::default(),
    ))
  }

  /// Constructs Behavior with a simple stateless handler.
  ///
  /// Handler always returns `BehaviorDirective::Same`.
  ///
  /// # Arguments
  /// * `handler` - Processing when message is received
  pub fn stateless<F>(mut handler: F) -> Self
  where
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) + 'static, {
    Self::Receive(BehaviorState::new_simple(
      Box::new(move |ctx, msg| {
        handler(ctx, msg);
        BehaviorDirective::Same
      }),
      SupervisorStrategyConfig::default(),
    ))
  }

  /// Constructs Behavior with a handler that receives only the message, without Context.
  ///
  /// # Arguments
  /// * `handler` - Processing when message is received
  pub fn receive_message<F>(mut handler: F) -> Self
  where
    F: FnMut(U) -> BehaviorDirective<U, R> + 'static, {
    Self::receive(move |_, msg| handler(msg))
  }

  /// Constructs Behavior with a handler that may fail and receives only the message.
  pub fn try_receive_message<F, E>(mut handler: F) -> Self
  where
    F: FnMut(U) -> Result<BehaviorDirective<U, R>, E> + 'static,
    E: fmt::Display + fmt::Debug + Send + 'static, {
    Self::try_receive(move |_, msg| handler(msg))
  }

  /// Creates a Behavior in stopped state.
  pub fn stopped() -> Self {
    Self::Stopped
  }

  /// Executes setup processing to generate Behavior.
  ///
  /// # Arguments
  /// * `init` - Initialization processing. Receives Context and returns Behavior
  pub fn setup<F>(init: F) -> Self
  where
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Behavior<U, R> + 'static, {
    let handler: Arc<SetupFn<U, R>> = Arc::new(init);
    Self::Setup {
      init: Some(ArcShared::from_arc(handler)),
      try_init: None,
      signal: None,
    }
  }

  /// Executes setup processing that may fail to generate Behavior.
  pub fn try_setup<F, E>(init: F) -> Self
  where
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Result<Behavior<U, R>, E> + 'static,
    E: fmt::Display + fmt::Debug + Send + 'static, {
    let handler: Arc<TrySetupFn<U, R>> = Arc::new(move |ctx| init(ctx).map_err(ActorFailure::from_error));
    Self::Setup {
      init: None,
      try_init: Some(ArcShared::from_arc(handler)),
      signal: None,
    }
  }

  /// Gets supervisor configuration (internal API).
  pub(crate) fn supervisor_config(&self) -> SupervisorStrategyConfig {
    match self {
      Behavior::Receive(state) => state.supervisor.clone(),
      Behavior::Setup { .. } | Behavior::Stopped => SupervisorStrategyConfig::default(),
    }
  }

  /// Adds a signal handler.
  ///
  /// # Arguments
  /// * `handler` - Processing when signal is received
  pub fn receive_signal<F>(self, handler: F) -> Self
  where
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>, Signal) -> BehaviorDirective<U, R> + 'static, {
    let handler: Arc<SignalFn<U, R>> = Arc::new(handler);
    let handler = ArcShared::from_arc(handler);
    self.attach_signal_arc(Some(handler))
  }

  fn attach_signal_arc(mut self, handler: Option<ArcShared<SignalFn<U, R>>>) -> Self {
    if let Some(handler) = handler {
      match &mut self {
        Behavior::Receive(state) => {
          state.set_signal_handler(handler);
        }
        Behavior::Setup { signal, .. } => {
          *signal = Some(handler);
        }
        Behavior::Stopped => {}
      }
    }
    self
  }
}

/// Behavior DSL builder.
///
/// Provides Akka Typed-style Behavior construction API.
pub struct Behaviors;

impl Behaviors {
  /// Constructs Behavior with specified message receive handler.
  pub fn receive<U, R, F>(handler: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone,
    R::Concurrency: MetadataStorageMode,
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> BehaviorDirective<U, R> + 'static, {
    Behavior::receive(handler)
  }

  /// Constructs Behavior with a handler that may fail.
  pub fn try_receive<U, R, F, E>(handler: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone,
    R::Concurrency: MetadataStorageMode,
    F: for<'r, 'ctx> FnMut(&mut Context<'r, 'ctx, U, R>, U) -> Result<BehaviorDirective<U, R>, E> + 'static,
    E: fmt::Display + fmt::Debug + Send + 'static, {
    Behavior::try_receive(handler)
  }

  /// Returns a directive to maintain current Behavior.
  pub fn same<U, R>() -> BehaviorDirective<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone, {
    BehaviorDirective::Same
  }

  /// Constructs Behavior with a handler that receives only the message.
  pub fn receive_message<U, R, F>(handler: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone,
    R::Concurrency: MetadataStorageMode,
    F: FnMut(U) -> BehaviorDirective<U, R> + 'static, {
    Behavior::receive_message(handler)
  }

  /// Constructs Behavior with a message-only handler that may fail.
  pub fn try_receive_message<U, R, F, E>(handler: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone,
    R::Concurrency: MetadataStorageMode,
    F: FnMut(U) -> Result<BehaviorDirective<U, R>, E> + 'static,
    E: fmt::Display + fmt::Debug + Send + 'static, {
    Behavior::try_receive_message(handler)
  }

  /// Returns a directive to transition to a new Behavior.
  pub fn transition<U, R>(behavior: Behavior<U, R>) -> BehaviorDirective<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone, {
    BehaviorDirective::Become(behavior)
  }

  /// Returns a directive to transition to stopped state.
  pub fn stopped<U, R>() -> BehaviorDirective<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone, {
    BehaviorDirective::Become(Behavior::stopped())
  }

  /// Creates a builder to set supervisor strategy on Behavior.
  pub fn supervise<U, R>(behavior: Behavior<U, R>) -> SuperviseBuilder<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone, {
    SuperviseBuilder { behavior }
  }

  /// Executes setup processing to generate Behavior.
  pub fn setup<U, R, F>(init: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone,
    R::Concurrency: MetadataStorageMode,
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Behavior<U, R> + 'static, {
    Behavior::setup(init)
  }

  /// Executes setup processing that may fail to generate Behavior.
  pub fn try_setup<U, R, F, E>(init: F) -> Behavior<U, R>
  where
    U: Element,
    R: ActorRuntime + Clone + 'static,
    R::Queue<PriorityEnvelope<DynMessage>>: Clone,
    R::Signal: Clone,
    R::Concurrency: MetadataStorageMode,
    F: for<'r, 'ctx> Fn(&mut Context<'r, 'ctx, U, R>) -> Result<Behavior<U, R>, E> + 'static,
    E: fmt::Display + fmt::Debug + Send + 'static, {
    Behavior::try_setup(init)
  }
}

/// Builder for setting supervisor strategy.
pub struct SuperviseBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  behavior: Behavior<U, R>,
}

impl<U, R> SuperviseBuilder<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Concurrency: MetadataStorageMode,
{
  /// Sets supervisor strategy.
  ///
  /// # Arguments
  /// * `strategy` - Supervisor strategy to apply
  pub fn with_strategy(mut self, strategy: SupervisorStrategy) -> Behavior<U, R> {
    if let Behavior::Receive(state) = &mut self.behavior {
      state.supervisor = SupervisorStrategyConfig::from_strategy(strategy);
    }
    self.behavior
  }
}

/// Adapter that bridges Behavior to untyped runtime.
///
/// Type used by internal runtime to connect Behavior and message dispatching.
pub struct ActorAdapter<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone, {
  behavior_factory: ArcShared<dyn Fn() -> Behavior<U, R> + 'static>,
  pub(super) behavior: Behavior<U, R>,
  pub(super) system_handler: Option<Box<SystemHandlerFn<U, R>>>,
}

impl<U, R> ActorAdapter<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<DynMessage>>: Clone,
  R::Signal: Clone,
  R::Concurrency: MetadataStorageMode,
{
  /// Creates a new `ActorAdapter`.
  ///
  /// # Arguments
  /// * `behavior_factory` - Factory function to create Behavior
  /// * `system_handler` - System message handler (optional)
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
  ///
  /// # Arguments
  /// * `ctx` - Actor context
  /// * `message` - Message to process
  pub fn handle_user(&mut self, ctx: &mut Context<'_, '_, U, R>, message: U) -> Result<(), ActorFailure> {
    self.ensure_initialized(ctx)?;
    match &mut self.behavior {
      Behavior::Receive(state) => match state.handle(ctx, message)? {
        BehaviorDirective::Same => {}
        BehaviorDirective::Become(next) => self.transition(next, ctx)?,
      },
      Behavior::Stopped => {
        // 処理不要
      }
      Behavior::Setup { .. } => unreachable!(),
    }
    Ok(())
  }

  /// Processes a system message.
  ///
  /// # Arguments
  /// * `ctx` - Actor context
  /// * `message` - System message to process
  pub fn handle_system(&mut self, ctx: &mut Context<'_, '_, U, R>, message: SystemMessage) -> Result<(), ActorFailure> {
    self.ensure_initialized(ctx)?;
    if matches!(message, SystemMessage::Stop) {
      self.transition(Behavior::stopped(), ctx)?;
    } else if matches!(message, SystemMessage::Restart) {
      self.behavior = (self.behavior_factory)();
      self.ensure_initialized(ctx)?;
    }
    if let Some(handler) = self.system_handler.as_mut() {
      handler(ctx, message);
    }
    Ok(())
  }

  /// Creates a SystemMessage mapper for Guardian/Scheduler.
  pub fn create_map_system() -> MapSystemShared<DynMessage> {
    MapSystemShared::new(|sys| DynMessage::new(MessageEnvelope::<U>::System(sys)))
  }

  /// Gets supervisor configuration (internal API).
  pub(crate) fn supervisor_config(&self) -> SupervisorStrategyConfig {
    self.behavior.supervisor_config()
  }

  fn ensure_initialized(&mut self, ctx: &mut Context<'_, '_, U, R>) -> Result<(), ActorFailure> {
    loop {
      match &self.behavior {
        Behavior::Setup { init, try_init, signal } => {
          let signal = signal.clone();
          if let Some(init) = init.clone() {
            let behavior = init(ctx);
            self.behavior = behavior.attach_signal_arc(signal);
          } else if let Some(init) = try_init.clone() {
            let behavior = init(ctx)?;
            self.behavior = behavior.attach_signal_arc(signal);
          } else {
            self.behavior = Behavior::stopped().attach_signal_arc(signal);
          }
        }
        _ => break,
      }
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
