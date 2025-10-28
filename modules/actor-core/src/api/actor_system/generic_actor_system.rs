use alloc::boxed::Box;
use core::{convert::Infallible, future::Future, marker::PhantomData, num::NonZeroUsize, pin::Pin};

use cellex_utils_core_rs::{
  collections::{queue::backend::QueueError, Element},
  sync::ArcShared,
};

use crate::{
  api::{
    actor::{actor_ref::PriorityActorRef, RootContext, ShutdownToken},
    actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf},
    actor_scheduler::ready_queue_scheduler::ReadyQueueWorker,
    actor_system::{ActorSystem, GenericActorSystemBuilder, GenericActorSystemConfig, GenericActorSystemRunner},
    extensions::{serializer_extension_id, Extension, ExtensionId, Extensions, SerializerRegistryExtension},
    failure::{
      failure_event_stream::FailureEventStream,
      failure_telemetry::{default_failure_telemetry_shared, FailureTelemetryContext},
    },
    guardian::AlwaysRestart,
    process::{
      pid::{NodeId, SystemId},
      process_registry::ProcessRegistry,
    },
  },
  internal::actor_system::{InternalActorSystem, InternalGenericActorSystemConfig},
  shared::{mailbox::messages::PriorityEnvelope, messaging::AnyMessage},
};

type GenericActorProcessRegistryHandle<AR> =
  ArcShared<ProcessRegistry<PriorityActorRef<AnyMessage, MailboxOf<AR>>, ArcShared<PriorityEnvelope<AnyMessage>>>>;

/// Primary instance of the actor system.
///
/// Responsible for actor spawning, management, and message dispatching.
pub struct GenericActorSystem<U, AR, Strat = AlwaysRestart>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>, {
  inner:                    InternalActorSystem<AR, Strat>,
  pub(crate) shutdown:      ShutdownToken,
  extensions:               Extensions,
  ready_queue_worker_count: NonZeroUsize,
  system_id:                SystemId,
  node_id:                  Option<NodeId>,
  _marker:                  PhantomData<U>,
}

impl<U, AR> GenericActorSystem<U, AR>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
{
  /// Creates a new actor system with an explicit runtime and configuration.
  ///
  /// # Panics
  ///
  /// This function contains an `expect` call that should never panic in practice,
  /// as it uses `NonZeroUsize::new(1)` which is guaranteed to succeed.
  #[allow(clippy::needless_pass_by_value)]
  pub fn new_with_actor_runtime(actor_runtime: AR, config: GenericActorSystemConfig<AR>) -> Self {
    let root_listener_from_runtime = actor_runtime.root_failure_event_listener_opt();
    let root_handler_from_runtime = actor_runtime.root_escalation_failure_event_handler_opt();
    let metrics_from_runtime = actor_runtime.metrics_sink_shared_opt();
    let scheduler_builder = actor_runtime.scheduler_builder_shared_builder_shared();

    let system_id = config.system_id().clone();
    let node_id = config.node_id_opt();

    let extensions_handle = config.extensions();
    if extensions_handle.get(serializer_extension_id()).is_none() {
      let extension = ArcShared::new(SerializerRegistryExtension::new());
      extensions_handle.register(extension);
    }
    let extensions = extensions_handle;

    let receive_timeout_scheduler_factory_shared_opt = config
      .receive_timeout_scheduler_factory_shared_opt()
      .or(actor_runtime.receive_timeout_scheduler_factory_shared_opt())
      .or_else(|| {
        actor_runtime.receive_timeout_scheduler_factory_provider_shared_opt().map(|driver| driver.build_factory())
      });
    let root_event_listener = config.failure_event_listener_opt().or(root_listener_from_runtime);
    let metrics_sink = config.metrics_sink_shared_opt().or(metrics_from_runtime);
    let telemetry_builder = config.failure_telemetry_builder_shared_opt();
    let root_failure_telemetry = if let Some(builder) = telemetry_builder {
      let ctx = FailureTelemetryContext::new(metrics_sink.clone(), extensions.clone());
      builder.build(&ctx)
    } else {
      config.failure_telemetry_shared_opt().unwrap_or_else(default_failure_telemetry_shared)
    };

    let mut observation_config = config.failure_observation_config_opt().unwrap_or_default();
    if let Some(sink) = metrics_sink.clone() {
      if observation_config.metrics_sink().is_none() {
        observation_config.set_metrics_sink(Some(sink));
      }
    }

    let settings = InternalGenericActorSystemConfig {
      root_event_listener_opt: root_event_listener,
      root_escalation_handler_opt: root_handler_from_runtime,
      receive_timeout_scheduler_factory_shared_opt,
      metrics_sink_opt: metrics_sink,
      root_failure_telemetry_shared: root_failure_telemetry,
      root_observation_config: observation_config,
      extensions: extensions.clone(),
      system_id: system_id.clone(),
      node_id_opt: node_id.clone(),
    };

    let ready_queue_worker_count = config
      .ready_queue_worker_count_opt()
      // SAFETY: NonZeroUsize::new(1) is always Some(1)
      .unwrap_or_else(|| unsafe { NonZeroUsize::new_unchecked(1) });

    Self {
      inner: InternalActorSystem::new_with_config_and_builder(actor_runtime, &scheduler_builder, settings),
      shutdown: ShutdownToken::default(),
      extensions,
      ready_queue_worker_count,
      system_id,
      node_id,
      _marker: PhantomData,
    }
  }

  /// Creates an actor system using the provided runtime and failure event stream.
  pub fn new_with_actor_runtime_and_event_stream<E>(actor_runtime: AR, event_stream: &E) -> Self
  where
    E: FailureEventStream, {
    let config = GenericActorSystemConfig::default().with_failure_event_listener_opt(Some(event_stream.listener()));
    Self::new_with_actor_runtime(actor_runtime, config)
  }

  /// Returns a builder that creates an actor system from the provided runtime preset.
  #[must_use]
  pub fn builder(actor_runtime: AR) -> GenericActorSystemBuilder<U, AR> {
    GenericActorSystemBuilder::new(actor_runtime)
  }
}

impl<U, AR, Strat> GenericActorSystem<U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>,
{
  /// Gets the shutdown token.
  ///
  /// # Returns
  /// Clone of the shutdown token
  #[must_use]
  pub fn shutdown_token(&self) -> ShutdownToken {
    self.shutdown.clone()
  }

  /// Converts this system into a runner.
  ///
  /// The runner provides an interface suitable for execution on an asynchronous runtime.
  ///
  /// # Returns
  /// Actor system runner
  #[must_use]
  #[allow(clippy::missing_const_for_fn)]
  pub fn into_runner(self) -> GenericActorSystemRunner<U, AR, Strat> {
    let ready_queue_worker_count = self.ready_queue_worker_count;
    GenericActorSystemRunner { system: self, ready_queue_worker_count, _marker: PhantomData }
  }

  /// Gets the root context.
  ///
  /// The root context is used to spawn actors at the top level of the actor system.
  ///
  /// # Returns
  /// Mutable reference to the root context
  pub fn root_context(&mut self) -> RootContext<'_, U, AR, Strat> {
    RootContext { inner: self.inner.root_context(), _marker: PhantomData }
  }

  /// Returns a clone of the shared extension registry.
  #[must_use]
  pub fn extensions(&self) -> Extensions {
    self.extensions.clone()
  }

  /// Applies a closure to the specified extension and returns its result.
  pub fn extension<E, F, T>(&self, id: ExtensionId, f: F) -> Option<T>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> T, {
    self.extensions.with::<E, _, _>(id, f)
  }

  /// Registers an extension handle with the running actor system.
  pub fn register_extension<E>(&self, extension: ArcShared<E>)
  where
    E: Extension + 'static, {
    self.extensions.register(extension);
  }

  /// Registers a dynamically typed extension handle with the running actor system.
  pub fn register_extension_dyn(&self, extension: ArcShared<dyn Extension>) {
    self.extensions.register_dyn(extension);
  }

  /// Registers an extension value by wrapping it with `ArcShared`.
  pub fn register_extension_value<E>(&self, extension: E)
  where
    E: Extension + 'static, {
    self.register_extension(ArcShared::new(extension));
  }

  /// Executes message dispatching until the specified condition is met.
  ///
  /// # Arguments
  /// * `should_continue` - Closure that determines continuation condition. Continues execution
  ///   while it returns `true`
  ///
  /// # Returns
  /// `Ok(())` on normal completion, `Err` on queue error
  ///
  /// # Errors
  /// Returns [`QueueError`] when dispatching an actor fails.
  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>
  where
    F: FnMut() -> bool, {
    self.inner.run_until(should_continue).await
  }

  /// Executes message dispatching permanently.
  ///
  /// This function does not terminate normally. Returns only on error.
  ///
  /// # Returns
  /// `Infallible` (does not terminate normally) or queue error
  ///
  /// # Errors
  /// Returns [`QueueError`] when dispatching an actor fails.
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>> {
    self.inner.run_forever().await
  }

  /// Dispatches one next message.
  ///
  /// Waits until a new message arrives if the queue is empty.
  ///
  /// # Returns
  /// `Ok(())` on normal completion, `Err` on queue error
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue processing fails.
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    self.inner.dispatch_next().await
  }

  /// Synchronously processes messages accumulated in the Ready queue, repeating until empty.
  /// Does not wait for new messages to arrive.
  ///
  /// # Errors
  /// Returns [`QueueError`] when queue processing fails.
  pub fn run_until_idle(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    let shutdown = self.shutdown.clone();
    self.inner.run_until_idle(|| !shutdown.is_triggered())
  }

  /// Returns a ReadyQueue worker handle if supported by the underlying scheduler.
  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>> {
    self.inner.ready_queue_worker()
  }

  /// Indicates whether the scheduler supports ReadyQueue-based execution.
  #[must_use]
  pub fn supports_ready_queue(&self) -> bool {
    self.ready_queue_worker().is_some()
  }

  /// Returns the process registry associated with this actor system.
  #[must_use]
  pub fn process_registry(&self) -> GenericActorProcessRegistryHandle<AR> {
    self.inner.process_registry()
  }

  /// Returns the system identifier assigned to this actor system.
  #[must_use]
  pub const fn system_id(&self) -> &SystemId {
    &self.system_id
  }

  /// Returns the node identifier when it has been configured.
  #[must_use]
  pub const fn node_id(&self) -> Option<&NodeId> {
    self.node_id.as_ref()
  }
}

impl<U, AR, Strat> ActorSystem<U, AR, Strat> for GenericActorSystem<U, AR, Strat>
where
  U: Element,
  AR: ActorRuntime + Clone + 'static,
  MailboxQueueOf<AR, PriorityEnvelope<AnyMessage>>: Clone,
  MailboxSignalOf<AR>: Clone,
  Strat: crate::api::guardian::GuardianStrategy<MailboxOf<AR>>,
{
  fn shutdown_token(&self) -> ShutdownToken {
    GenericActorSystem::shutdown_token(self)
  }

  fn root_context(&mut self) -> RootContext<'_, U, AR, Strat> {
    GenericActorSystem::root_context(self)
  }

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<MailboxOf<AR>>>> {
    GenericActorSystem::ready_queue_worker(self)
  }

  fn supports_ready_queue(&self) -> bool {
    GenericActorSystem::supports_ready_queue(self)
  }

  fn run_until_idle(&mut self) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> {
    GenericActorSystem::run_until_idle(self)
  }

  fn run_until<'a, F>(
    &'a mut self,
    should_continue: F,
  ) -> Pin<Box<dyn Future<Output = Result<(), QueueError<PriorityEnvelope<AnyMessage>>>> + 'a>>
  where
    F: FnMut() -> bool + 'a, {
    Box::pin(GenericActorSystem::run_until(self, should_continue))
  }

  fn run_forever(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = Result<Infallible, QueueError<PriorityEnvelope<AnyMessage>>>> + '_>> {
    Box::pin(GenericActorSystem::run_forever(self))
  }

  fn dispatch_next(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = Result<(), QueueError<PriorityEnvelope<AnyMessage>>>> + '_>> {
    Box::pin(GenericActorSystem::dispatch_next(self))
  }
}
