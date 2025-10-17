use crate::api::actor_system::actor_system_builder::ActorSystemBuilder;
use crate::api::actor_system::actor_system_config::ActorSystemConfig;
use crate::api::actor_system::actor_system_runner::ActorSystemRunner;
use core::convert::Infallible;
use core::marker::PhantomData;
use core::num::NonZeroUsize;

use crate::api::actor::root_context::RootContext;
use crate::api::actor::shutdown_token::ShutdownToken;
use crate::api::actor_runtime::{ActorRuntime, MailboxOf, MailboxQueueOf, MailboxSignalOf};
use crate::api::extensions::serializer_extension_id;
use crate::api::extensions::Extension;
use crate::api::extensions::ExtensionId;
use crate::api::extensions::Extensions;
use crate::api::extensions::SerializerRegistryExtension;
use crate::api::failure_event_stream::FailureEventStream;
use crate::api::mailbox::PriorityEnvelope;
use crate::api::messaging::DynMessage;
use crate::api::supervision::telemetry::default_failure_telemetry_shared;
use crate::internal::actor_system::{InternalActorSystem, InternalActorSystemConfig};
use crate::internal::guardian::AlwaysRestart;
use crate::internal::scheduler::ReadyQueueWorker;
use crate::shared::failure_telemetry::TelemetryContext;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};

/// Primary instance of the actor system.
///
/// Responsible for actor spawning, management, and message dispatching.
pub struct ActorSystem<U, R, Strat = AlwaysRestart>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  Strat: crate::internal::guardian::GuardianStrategy<DynMessage, MailboxOf<R>>, {
  inner: InternalActorSystem<DynMessage, R, Strat>,
  pub(crate) shutdown: ShutdownToken,
  extensions: Extensions,
  ready_queue_worker_count: NonZeroUsize,
  _marker: PhantomData<U>,
}

impl<U, R> ActorSystem<U, R>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
{
  /// Creates a new actor system with an explicit runtime and configuration.
  ///
  /// # Panics
  ///
  /// This function contains an `expect` call that should never panic in practice,
  /// as it uses `NonZeroUsize::new(1)` which is guaranteed to succeed.
  #[allow(clippy::needless_pass_by_value)]
  pub fn new_with_actor_runtime(actor_runtime: R, config: ActorSystemConfig<R>) -> Self {
    let root_listener_from_runtime = actor_runtime.root_event_listener_opt();
    let root_handler_from_runtime = actor_runtime.root_escalation_handler_opt();
    let metrics_from_runtime = actor_runtime.metrics_sink_shared_opt();
    let scheduler_builder = actor_runtime.scheduler_builder_shared();

    let extensions_handle = config.extensions();
    if extensions_handle.get(serializer_extension_id()).is_none() {
      let extension = ArcShared::new(SerializerRegistryExtension::new());
      extensions_handle.register(extension);
    }
    let extensions = extensions_handle;

    let receive_timeout_factory = config
      .receive_timeout_scheduler_factory_shared()
      .or(actor_runtime.receive_timeout_scheduler_factory_shared_opt())
      .or_else(|| {
        actor_runtime
          .receive_timeout_scheduler_factory_provider_shared_opt()
          .map(|driver| driver.build_factory())
      });
    let root_event_listener = config.failure_event_listener().or(root_listener_from_runtime);
    let metrics_sink = config.metrics_sink_shared().or(metrics_from_runtime);
    let telemetry_builder = config.failure_telemetry_builder_shared();
    let root_failure_telemetry = if let Some(builder) = telemetry_builder {
      let ctx = TelemetryContext::new(metrics_sink.clone(), extensions.clone());
      builder.build(&ctx)
    } else {
      config
        .failure_telemetry_shared()
        .unwrap_or_else(default_failure_telemetry_shared)
    };

    let mut observation_config = config.failure_observation_config().unwrap_or_default();
    if let Some(sink) = metrics_sink.clone() {
      if observation_config.metrics_sink().is_none() {
        observation_config.set_metrics_sink(Some(sink));
      }
      #[cfg(feature = "std")]
      {
        if !observation_config.should_record_timing() {
          observation_config.set_record_timing(true);
        }
      }
    }

    let settings = InternalActorSystemConfig {
      root_event_listener,
      root_escalation_handler: root_handler_from_runtime,
      receive_timeout_factory,
      metrics_sink,
      root_failure_telemetry,
      root_observation_config: observation_config,
      extensions: extensions.clone(),
    };

    let ready_queue_worker_count = config
      .ready_queue_worker_count()
      // SAFETY: NonZeroUsize::new(1) is always Some(1)
      .unwrap_or_else(|| unsafe { NonZeroUsize::new_unchecked(1) });

    Self {
      inner: InternalActorSystem::new_with_settings_and_builder(actor_runtime, &scheduler_builder, settings),
      shutdown: ShutdownToken::default(),
      extensions,
      ready_queue_worker_count,
      _marker: PhantomData,
    }
  }

  /// Creates an actor system using the provided runtime and failure event stream.
  pub fn new_with_actor_runtime_and_event_stream<E>(actor_runtime: R, event_stream: &E) -> Self
  where
    E: FailureEventStream, {
    let config = ActorSystemConfig::default().with_failure_event_listener(Some(event_stream.listener()));
    Self::new_with_actor_runtime(actor_runtime, config)
  }

  /// Returns a builder that creates an actor system from the provided runtime preset.
  #[must_use]
  pub fn builder(actor_runtime: R) -> ActorSystemBuilder<U, R> {
    ActorSystemBuilder::new(actor_runtime)
  }
}

impl<U, R, Strat> ActorSystem<U, R, Strat>
where
  U: Element,
  R: ActorRuntime + Clone + 'static,
  MailboxQueueOf<R, PriorityEnvelope<DynMessage>>: Clone,
  MailboxSignalOf<R>: Clone,
  Strat: crate::internal::guardian::GuardianStrategy<DynMessage, MailboxOf<R>>,
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
  pub fn into_runner(self) -> ActorSystemRunner<U, R, Strat> {
    let ready_queue_worker_count = self.ready_queue_worker_count;
    ActorSystemRunner {
      system: self,
      ready_queue_worker_count,
      _marker: PhantomData,
    }
  }

  /// Gets the root context.
  ///
  /// The root context is used to spawn actors at the top level of the actor system.
  ///
  /// # Returns
  /// Mutable reference to the root context
  pub fn root_context(&mut self) -> RootContext<'_, U, R, Strat> {
    RootContext {
      inner: self.inner.root_context(),
      _marker: PhantomData,
    }
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
  /// * `should_continue` - Closure that determines continuation condition. Continues execution while it returns `true`
  ///
  /// # Returns
  /// `Ok(())` on normal completion, `Err` on queue error
  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>
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
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.run_forever().await
  }

  /// Dispatches one next message.
  ///
  /// Waits until a new message arrives if the queue is empty.
  ///
  /// # Returns
  /// `Ok(())` on normal completion, `Err` on queue error
  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    self.inner.dispatch_next().await
  }

  /// Synchronously processes messages accumulated in the Ready queue, repeating until empty.
  /// Does not wait for new messages to arrive.
  pub fn run_until_idle(&mut self) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> {
    let shutdown = self.shutdown.clone();
    self.inner.run_until_idle(|| !shutdown.is_triggered())
  }

  /// Returns a ReadyQueue worker handle if supported by the underlying scheduler.
  #[must_use]
  pub fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<DynMessage, MailboxOf<R>>>> {
    self.inner.ready_queue_worker()
  }

  /// Indicates whether the scheduler supports ReadyQueue-based execution.
  #[must_use]
  pub fn supports_ready_queue(&self) -> bool {
    self.ready_queue_worker().is_some()
  }
}
