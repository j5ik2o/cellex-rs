#![allow(missing_docs, deprecated)]

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::marker::PhantomData;

use spin::Mutex;

use async_trait::async_trait;

use crate::runtime::context::InternalActorRef;
use crate::runtime::guardian::{AlwaysRestart, Guardian, GuardianStrategy};
use crate::runtime::mailbox::traits::{Mailbox, MailboxProducer};
use crate::runtime::supervision::CompositeEscalationSink;
use crate::ActorId;
use crate::ActorPath;
use crate::{
  EscalationSink, Extensions, FailureEventHandler, FailureEventListener, FailureInfo, FailureTelemetryShared,
  Supervisor, TelemetryObservationConfig,
};
use crate::{MailboxRuntime, PriorityEnvelope};
use crate::{MailboxSignal, SystemMessage};
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError};
use futures::future::{select, select_all, Either, LocalBoxFuture};
use futures::FutureExt;

use super::actor_cell::ActorCell;
use super::actor_scheduler::{ActorScheduler, SchedulerBuilder, SchedulerSpawnContext};
use crate::{MapSystemShared, MetricsEvent, MetricsSinkShared, ReceiveTimeoutFactoryShared};

/// Hook invoked by mailboxes when new messages arrive.
#[cfg(target_has_atomic = "ptr")]
pub trait ReadyEventHook: Send + Sync {
  /// Notifies the scheduler that the associated actor has become ready.
  fn notify_ready(&self);
}

/// Hook invoked by mailboxes when new messages arrive (no atomic pointer targets).
#[cfg(not(target_has_atomic = "ptr"))]
pub trait ReadyEventHook {
  /// Notifies the scheduler that the associated actor has become ready.
  fn notify_ready(&self);
}

/// Shared handle to a [`ReadyEventHook`].
#[cfg(target_has_atomic = "ptr")]
pub type ReadyQueueHandle = ArcShared<dyn ReadyEventHook + Send + Sync>;

/// Shared handle to a [`ReadyEventHook`] (no atomic pointer targets).
#[cfg(not(target_has_atomic = "ptr"))]
pub type ReadyQueueHandle = ArcShared<dyn ReadyEventHook>;

/// Simple scheduler implementation assuming priority mailboxes.
pub struct PrioritySchedulerCore<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  pub(super) guardian: Guardian<M, R, Strat>,
  actors: Vec<ActorCell<M, R, Strat>>,
  escalations: Vec<FailureInfo>,
  escalation_sink: CompositeEscalationSink<M, R>,
  receive_timeout_factory: Option<ReceiveTimeoutFactoryShared<M, R>>,
  metrics_sink: Option<MetricsSinkShared>,
  extensions: Extensions,
  _strategy: PhantomData<Strat>,
}

#[allow(dead_code)]
impl<M, R> PrioritySchedulerCore<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
{
  pub fn new(runtime: R, extensions: Extensions) -> Self {
    Self::with_strategy(runtime, AlwaysRestart, extensions)
  }

  pub fn with_strategy<Strat>(
    _runtime: R,
    strategy: Strat,
    extensions: Extensions,
  ) -> PrioritySchedulerCore<M, R, Strat>
  where
    Strat: GuardianStrategy<M, R>, {
    PrioritySchedulerCore {
      guardian: Guardian::new(strategy),
      actors: Vec::new(),
      escalations: Vec::new(),
      escalation_sink: CompositeEscalationSink::default(),
      receive_timeout_factory: None,
      metrics_sink: None,
      extensions,
      _strategy: PhantomData,
    }
  }
}

#[deprecated(
  note = "Use ReadyQueueScheduler instead; PriorityScheduler remains only for legacy scenarios and will be removed in a future release."
)]
pub struct PriorityScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  core: PrioritySchedulerCore<M, R, Strat>,
}

#[allow(dead_code)]
pub struct ReadyQueueScheduler<M, R, Strat = AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  context: ArcShared<Mutex<ReadyQueueContext<M, R, Strat>>>,
  state: ArcShared<Mutex<ReadyQueueState>>,
}

struct ReadyQueueContext<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  core: PrioritySchedulerCore<M, R, Strat>,
  state: ArcShared<Mutex<ReadyQueueState>>,
}

struct ReadyQueueState {
  queue: VecDeque<usize>,
  queued: Vec<bool>,
  running: Vec<bool>,
}

impl ReadyQueueState {
  fn new() -> Self {
    Self {
      queue: VecDeque::new(),
      queued: Vec::new(),
      running: Vec::new(),
    }
  }

  fn ensure_capacity(&mut self, len: usize) {
    if self.queued.len() < len {
      self.queued.resize(len, false);
    }
    if self.running.len() < len {
      self.running.resize(len, false);
    }
  }

  fn enqueue_if_idle(&mut self, index: usize) -> bool {
    self.ensure_capacity(index + 1);
    if self.running[index] || self.queued[index] {
      return false;
    }
    self.queue.push_back(index);
    self.queued[index] = true;
    true
  }

  fn mark_running(&mut self, index: usize) {
    self.ensure_capacity(index + 1);
    self.running[index] = true;
    if index < self.queued.len() {
      self.queued[index] = false;
    }
  }

  fn mark_idle(&mut self, index: usize, has_pending: bool) {
    self.ensure_capacity(index + 1);
    self.running[index] = false;
    if has_pending {
      let _ = self.enqueue_if_idle(index);
    }
  }
}

impl<M, R, Strat> ReadyQueueContext<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  fn actor_count(&self) -> usize {
    self.core.actor_count()
  }

  fn actor_mut(&mut self, index: usize) -> Option<&mut ActorCell<M, R, Strat>> {
    self.core.actor_mut(index)
  }

  fn actor_has_pending(&self, index: usize) -> bool {
    self.core.actor_has_pending(index)
  }

  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<(InternalActorRef<M, R>, usize), QueueError<PriorityEnvelope<M>>> {
    let actor_ref = self.core.spawn_actor(supervisor, context)?;
    let index = self.core.actor_count().saturating_sub(1);
    Ok((actor_ref, index))
  }

  fn enqueue_ready(&self, index: usize) {
    let mut state = self.state.lock();
    let _ = state.enqueue_if_idle(index);
  }

  fn dequeue_ready(&self) -> Option<usize> {
    let mut state = self.state.lock();
    let index = state.queue.pop_front()?;
    state.queued[index] = false;
    state.mark_running(index);
    Some(index)
  }

  fn mark_idle(&self, index: usize, has_pending: bool) {
    let mut state = self.state.lock();
    state.mark_idle(index, has_pending);
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.core.drain_ready()
  }

  fn process_actor_pending(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.core.process_actor_pending(index)
  }

  fn wait_for_any_signal_future(&self) -> Option<LocalBoxFuture<'static, usize>> {
    self.core.wait_for_any_signal_future()
  }

  fn process_ready_once(&mut self) -> Result<Option<bool>, QueueError<PriorityEnvelope<M>>> {
    if let Some(index) = self.dequeue_ready() {
      let processed = self.core.process_actor_pending(index)?;
      let has_pending = self.actor_has_pending(index);
      self.mark_idle(index, has_pending);
      return Ok(Some(processed));
    }

    if self.core.drain_ready()? {
      return Ok(Some(true));
    }

    Ok(None)
  }

  fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    self.core.on_escalation(handler)
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.core.take_escalations()
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) {
    self.core.set_receive_timeout_factory(factory)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.core.set_metrics_sink(sink)
  }

  fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    self.core.set_parent_guardian(control_ref, map_system)
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.core.set_root_event_listener(listener)
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    self.core.set_root_escalation_handler(handler)
  }

  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.core.set_root_failure_telemetry(telemetry)
  }

  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    self.core.set_root_observation_config(config)
  }
}

#[allow(dead_code)]
impl<M, R> PriorityScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
{
  pub fn new(runtime: R, extensions: Extensions) -> Self {
    Self::with_strategy(runtime, AlwaysRestart, extensions)
  }

  pub fn with_strategy<Strat>(runtime: R, strategy: Strat, extensions: Extensions) -> PriorityScheduler<M, R, Strat>
  where
    Strat: GuardianStrategy<M, R>, {
    PriorityScheduler {
      core: PrioritySchedulerCore::with_strategy(runtime, strategy, extensions),
    }
  }
}

#[allow(dead_code)]
impl<M, R> ReadyQueueScheduler<M, R, AlwaysRestart>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
{
  pub fn new(runtime: R, extensions: Extensions) -> Self {
    Self::with_strategy(runtime, AlwaysRestart, extensions)
  }

  pub fn with_strategy<Strat>(runtime: R, strategy: Strat, extensions: Extensions) -> ReadyQueueScheduler<M, R, Strat>
  where
    Strat: GuardianStrategy<M, R>, {
    let state = ArcShared::new(Mutex::new(ReadyQueueState::new()));
    let context = ReadyQueueContext {
      core: PrioritySchedulerCore::with_strategy(runtime, strategy, extensions),
      state: state.clone(),
    };
    ReadyQueueScheduler {
      context: ArcShared::new(Mutex::new(context)),
      state,
    }
  }
}

struct ReadyNotifier {
  state: ArcShared<Mutex<ReadyQueueState>>,
  index: usize,
}

impl ReadyNotifier {
  fn new(state: ArcShared<Mutex<ReadyQueueState>>, index: usize) -> Self {
    Self { state, index }
  }
}

impl ReadyEventHook for ReadyNotifier {
  fn notify_ready(&self) {
    let mut state = self.state.lock();
    let _ = state.enqueue_if_idle(self.index);
  }
}

impl<M, R> SchedulerBuilder<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
{
  pub fn priority() -> Self {
    Self::new(|runtime, extensions| Box::new(ReadyQueueScheduler::new(runtime, extensions)))
  }

  #[allow(dead_code)]
  pub fn with_strategy<Strat>(self, strategy: Strat) -> Self
  where
    Strat: GuardianStrategy<M, R> + Clone + Send + Sync, {
    let _ = self;
    Self::new(move |runtime, extensions| {
      Box::new(ReadyQueueScheduler::with_strategy(
        runtime,
        strategy.clone(),
        extensions,
      ))
    })
  }
}

#[allow(dead_code)]
impl<M, R, Strat> PrioritySchedulerCore<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  pub fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    let SchedulerSpawnContext {
      runtime,
      mailbox_handle_factory_stub: mailbox_factory,
      map_system,
      mailbox_options,
      handler,
    } = context;
    let mailbox_factory = mailbox_factory.with_metrics_sink(self.metrics_sink.clone());
    let mut mailbox_spawner = mailbox_factory.priority_spawner();
    mailbox_spawner.set_metrics_sink(self.metrics_sink.clone());
    let (mut mailbox, mut sender) = mailbox_spawner.spawn_mailbox(mailbox_options);
    mailbox.set_metrics_sink(self.metrics_sink.clone());
    sender.set_metrics_sink(self.metrics_sink.clone());
    let actor_sender = sender.clone();
    let control_ref = InternalActorRef::new(actor_sender.clone());
    let watchers = vec![ActorId::ROOT];
    let primary_watcher = watchers.first().copied();
    let parent_path = ActorPath::new();
    let (actor_id, actor_path) =
      self
        .guardian
        .register_child(control_ref.clone(), map_system.clone(), primary_watcher, &parent_path)?;
    let mut cell = ActorCell::new(
      actor_id,
      map_system,
      watchers,
      actor_path,
      runtime,
      mailbox_spawner,
      mailbox,
      sender,
      supervisor,
      handler,
      self.receive_timeout_factory.clone(),
      self.extensions.clone(),
    );
    cell.set_metrics_sink(self.metrics_sink.clone());
    self.actors.push(cell);
    self.record_metric(MetricsEvent::ActorRegistered);
    Ok(control_ref)
  }

  /// Legacy sync API. Internally uses the same path as `dispatch_next`,
  /// but `run_until` / `dispatch_next` is recommended for new code.
  #[deprecated(since = "3.1.0", note = "Use dispatch_next / run_until instead")]
  pub fn dispatch_all(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    #[cfg(feature = "std")]
    {
      use core::sync::atomic::{AtomicBool, Ordering};
      static WARNED: AtomicBool = AtomicBool::new(false);
      if !WARNED.swap(true, Ordering::Relaxed) {
        tracing::warn!(
          "PriorityScheduler::dispatch_all is deprecated. Consider using dispatch_next / run_until instead."
        );
      }
    }
    let _ = self.drain_ready_cycle()?;
    Ok(())
  }

  /// Helper that repeats `dispatch_next` as long as the condition holds.
  /// Allows simple construction of wait loops that can be controlled from the runtime side.
  pub async fn run_until<F>(&mut self, mut should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    while should_continue() {
      self.dispatch_next().await?;
    }
    Ok(())
  }

  /// Runs the scheduler as a resident async task. Can be used like
  /// `tokio::spawn(async move { scheduler.run_forever().await })`.
  /// Stops on error or task cancellation.
  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    loop {
      self.dispatch_next().await?;
    }
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    loop {
      if self.drain_ready_cycle()? {
        return Ok(());
      }

      let Some(wait_future) = self.wait_for_any_signal_future() else {
        return Ok(());
      };
      let index = wait_future.await;

      if self.process_waiting_actor(index).await? {
        return Ok(());
      }
    }
  }

  pub fn actor_count(&self) -> usize {
    self.actors.len()
  }

  pub fn actor_mut(&mut self, index: usize) -> Option<&mut ActorCell<M, R, Strat>> {
    self.actors.get_mut(index)
  }

  pub fn actor_has_pending(&self, index: usize) -> bool {
    self
      .actors
      .get(index)
      .map(|cell| cell.has_pending_messages())
      .unwrap_or(false)
  }

  pub fn take_escalations(&mut self) -> Vec<FailureInfo> {
    core::mem::take(&mut self.escalations)
  }

  /// Processes one cycle of messages in the Ready queue. Returns true if processing occurred.
  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.drain_ready_cycle()
  }

  pub fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) {
    self.receive_timeout_factory = factory.clone();
    for actor in &mut self.actors {
      actor.configure_receive_timeout_factory(factory.clone());
    }
  }

  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.metrics_sink = sink.clone();
    for actor in &mut self.actors {
      actor.set_metrics_sink(sink.clone());
    }
  }

  pub fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    self.escalation_sink.set_custom_handler(handler);
  }

  pub fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    self.escalation_sink.set_parent_guardian(control_ref, map_system);
  }

  pub fn set_root_escalation_handler(&mut self, handler: Option<crate::FailureEventHandler>) {
    self.escalation_sink.set_root_handler(handler);
  }

  pub fn set_root_event_listener(&mut self, listener: Option<crate::FailureEventListener>) {
    self.escalation_sink.set_root_listener(listener);
  }

  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.escalation_sink.set_root_telemetry(telemetry);
  }

  pub fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    self.escalation_sink.set_root_observation_config(config);
  }

  fn handle_escalations(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if self.escalations.is_empty() {
      return Ok(false);
    }

    let pending = core::mem::take(&mut self.escalations);
    let mut remaining = Vec::new();
    let mut handled = false;
    for info in pending.into_iter() {
      let handled_locally = self.forward_to_local_parent(&info);
      match self.escalation_sink.handle(info, handled_locally) {
        Ok(()) => handled = true,
        Err(unhandled) => remaining.push(unhandled),
      }
    }
    self.escalations = remaining;
    Ok(handled)
  }

  fn wait_for_any_signal_future(&self) -> Option<LocalBoxFuture<'static, usize>> {
    if self.actors.is_empty() {
      return None;
    }

    let mut waiters = Vec::with_capacity(self.actors.len());
    for (idx, cell) in self.actors.iter().enumerate() {
      let signal = cell.signal_clone();
      waiters.push(
        async move {
          signal.wait().await;
          idx
        }
        .boxed_local(),
      );
    }

    Some(Box::pin(async move {
      let (idx, _, _) = select_all(waiters).await;
      idx
    }))
  }

  fn drain_ready_cycle(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    let mut new_children = Vec::new();
    let len = self.actors.len();
    let mut processed_any = false;
    for idx in 0..len {
      let cell = &mut self.actors[idx];
      let processed = cell.process_pending(&mut self.guardian, &mut new_children, &mut self.escalations)?;
      if processed > 0 {
        self.record_messages_dequeued(processed);
        processed_any = true;
      }
    }
    self.finish_cycle(new_children, processed_any)
  }

  async fn process_waiting_actor(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if index >= self.actors.len() {
      return Ok(false);
    }

    let mut new_children = Vec::new();
    let processed_count = self.actors[index]
      .wait_and_process(&mut self.guardian, &mut new_children, &mut self.escalations)
      .await?;
    if processed_count > 0 {
      self.record_messages_dequeued(processed_count);
    }

    self.finish_cycle(new_children, processed_count > 0)
  }

  pub fn process_actor_pending(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if index >= self.actors.len() {
      return Ok(false);
    }

    let mut new_children = Vec::new();
    let processed_count =
      self.actors[index].process_pending(&mut self.guardian, &mut new_children, &mut self.escalations)?;
    if processed_count > 0 {
      self.record_messages_dequeued(processed_count);
    }

    self.finish_cycle(new_children, processed_count > 0)
  }

  fn finish_cycle(
    &mut self,
    new_children: Vec<ActorCell<M, R, Strat>>,
    processed_any: bool,
  ) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    if !new_children.is_empty() {
      let added = new_children.len();
      self.actors.extend(new_children);
      self.record_repeated(MetricsEvent::ActorRegistered, added);
    }

    let handled = self.handle_escalations()?;
    let removed = self.prune_stopped();
    Ok(processed_any || handled || removed)
  }

  fn forward_to_local_parent(&self, info: &FailureInfo) -> bool {
    if let Some(parent_info) = info.escalate_to_parent() {
      if parent_info.path.is_empty() {
        return false;
      }

      if let Some((parent_ref, map_system)) = self.guardian.child_route(parent_info.actor) {
        #[allow(clippy::redundant_closure)]
        let envelope = PriorityEnvelope::from_system(SystemMessage::Escalate(parent_info)).map(|sys| (map_system)(sys));
        if parent_ref.sender().try_send(envelope).is_ok() {
          return true;
        }
      }
    }

    false
  }

  fn prune_stopped(&mut self) -> bool {
    let before = self.actors.len();
    self.actors.retain(|cell| !cell.is_stopped());
    let removed = before.saturating_sub(self.actors.len());
    if removed > 0 {
      self.record_repeated(MetricsEvent::ActorDeregistered, removed);
      return true;
    }
    false
  }

  fn record_metric(&self, event: MetricsEvent) {
    self.record_repeated(event, 1);
  }

  fn record_messages_dequeued(&self, count: usize) {
    self.record_repeated(MetricsEvent::MailboxDequeued, count);
  }

  fn record_repeated(&self, event: MetricsEvent, count: usize) {
    if count == 0 {
      return;
    }
    if let Some(sink) = &self.metrics_sink {
      sink.with_ref(|sink| {
        for _ in 0..count {
          sink.record(event);
        }
      });
    }
  }
}

impl<M, R, Strat> PriorityScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  fn core(&self) -> &PrioritySchedulerCore<M, R, Strat> {
    &self.core
  }

  fn core_mut(&mut self) -> &mut PrioritySchedulerCore<M, R, Strat> {
    &mut self.core
  }

  pub fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    self.core_mut().spawn_actor(supervisor, context)
  }

  pub async fn run_until<F>(&mut self, should_continue: F) -> Result<(), QueueError<PriorityEnvelope<M>>>
  where
    F: FnMut() -> bool, {
    self.core_mut().run_until(should_continue).await
  }

  pub async fn run_forever(&mut self) -> Result<Infallible, QueueError<PriorityEnvelope<M>>> {
    self.core_mut().run_forever().await
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    self.core_mut().dispatch_next().await
  }

  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.core_mut().drain_ready()
  }

  pub fn process_actor_pending(&mut self, index: usize) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    self.core_mut().process_actor_pending(index)
  }

  pub fn actor_count(&self) -> usize {
    self.core().actor_count()
  }

  pub fn take_escalations(&mut self) -> Vec<FailureInfo> {
    self.core_mut().take_escalations()
  }

  pub fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) {
    self.core_mut().set_receive_timeout_factory(factory)
  }

  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    self.core_mut().set_metrics_sink(sink)
  }

  pub fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    self.core_mut().on_escalation(handler)
  }

  pub fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    self.core_mut().set_parent_guardian(control_ref, map_system)
  }

  pub fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    self.core_mut().set_root_escalation_handler(handler)
  }

  pub fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    self.core_mut().set_root_event_listener(listener)
  }

  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    self.core_mut().set_root_failure_telemetry(telemetry)
  }

  pub fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    self.core_mut().set_root_observation_config(config)
  }
}

/// Worker interface exposing ReadyQueue operations for driver-level scheduling.
pub trait ReadyQueueWorker<M, R>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  /// Processes one ready actor (if any). Returns `Some(true)` if progress was made.
  fn process_ready_once(&self) -> Result<Option<bool>, QueueError<PriorityEnvelope<M>>>;

  /// Returns a future that resolves when any actor becomes ready.
  fn wait_for_ready(&self) -> Option<LocalBoxFuture<'static, usize>>;
}

/// Drives a single ReadyQueue worker loop until shutdown is triggered.
pub async fn drive_ready_queue_worker<M, R, Y, YF, S, SF>(
  worker: ArcShared<dyn ReadyQueueWorker<M, R>>,
  shutdown: ShutdownToken,
  mut yield_now: Y,
  mut wait_for_shutdown: S,
) -> Result<(), QueueError<PriorityEnvelope<M>>>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  Y: FnMut() -> YF,
  YF: core::future::Future<Output = ()>,
  S: FnMut() -> SF,
  SF: core::future::Future<Output = ()>, {
  loop {
    if shutdown.is_triggered() {
      return Ok(());
    }

    if let Some(progress) = worker.process_ready_once()? {
      if progress {
        yield_now().await;
        continue;
      }
    }

    match worker.wait_for_ready() {
      Some(wait_future) => {
        let shutdown_future = wait_for_shutdown();
        futures::pin_mut!(wait_future);
        futures::pin_mut!(shutdown_future);
        match select(wait_future, shutdown_future).await {
          Either::Left((_, _)) => {}
          Either::Right((_, _)) => return Ok(()),
        }
      }
      None => {
        yield_now().await;
      }
    }
  }
}

struct ReadyQueueWorkerImpl<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>, {
  context: ArcShared<Mutex<ReadyQueueContext<M, R, Strat>>>,
}

impl<M, R, Strat> ReadyQueueWorkerImpl<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  fn new(context: ArcShared<Mutex<ReadyQueueContext<M, R, Strat>>>) -> Self {
    Self { context }
  }
}

impl<M, R, Strat> ReadyQueueWorker<M, R> for ReadyQueueWorkerImpl<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  fn process_ready_once(&self) -> Result<Option<bool>, QueueError<PriorityEnvelope<M>>> {
    let mut ctx = self.context.lock();
    ctx.process_ready_once()
  }

  fn wait_for_ready(&self) -> Option<LocalBoxFuture<'static, usize>> {
    let ctx = self.context.lock();
    ctx.wait_for_any_signal_future()
  }
}

impl<M, R, Strat> ReadyQueueScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  pub fn worker_handle(&self) -> ArcShared<dyn ReadyQueueWorker<M, R>> {
    let shared = ArcShared::new(ReadyQueueWorkerImpl::<M, R, Strat>::new(self.context.clone()));
    shared.into_dyn(|inner| inner as &dyn ReadyQueueWorker<M, R>)
  }

  fn make_ready_handle(&self, index: usize) -> ReadyQueueHandle {
    let state = self.state.clone();
    let notifier = ArcShared::new(ReadyNotifier::new(state, index));
    #[cfg(target_has_atomic = "ptr")]
    {
      notifier.into_dyn(|inner| inner as &(dyn ReadyEventHook + Send + Sync))
    }
    #[cfg(not(target_has_atomic = "ptr"))]
    {
      notifier.into_dyn(|inner| inner as &dyn ReadyEventHook)
    }
  }

  pub fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    let (actor_ref, index) = {
      let mut ctx = self.context.lock();
      ctx.spawn_actor(supervisor, context)?
    };

    let hook = self.make_ready_handle(index);
    {
      let mut ctx = self.context.lock();
      if let Some(cell) = ctx.actor_mut(index) {
        cell.set_scheduler_hook(Some(hook));
      }
      ctx.enqueue_ready(index);
    }

    Ok(actor_ref)
  }

  pub fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) {
    let mut ctx = self.context.lock();
    ctx.set_receive_timeout_factory(factory);
  }

  pub fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    let mut ctx = self.context.lock();
    ctx.set_metrics_sink(sink);
  }

  pub fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    let mut ctx = self.context.lock();
    ctx.set_root_event_listener(listener);
  }

  pub fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    let mut ctx = self.context.lock();
    ctx.set_root_escalation_handler(handler);
  }

  pub fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    let mut ctx = self.context.lock();
    ctx.set_parent_guardian(control_ref, map_system);
  }

  pub fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    let mut ctx = self.context.lock();
    ctx.set_root_failure_telemetry(telemetry);
  }

  pub fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    let mut ctx = self.context.lock();
    ctx.set_root_observation_config(config);
  }

  pub fn on_escalation<F>(&mut self, handler: F)
  where
    F: FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static, {
    let mut ctx = self.context.lock();
    ctx.on_escalation(handler);
  }

  pub fn take_escalations(&mut self) -> Vec<FailureInfo> {
    let mut ctx = self.context.lock();
    ctx.take_escalations()
  }

  pub fn actor_count(&self) -> usize {
    let ctx = self.context.lock();
    ctx.actor_count()
  }

  pub fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    let mut ctx = self.context.lock();
    ctx.drain_ready()
  }

  pub async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    loop {
      {
        let mut ctx = self.context.lock();
        if let Some(index) = ctx.dequeue_ready() {
          let processed = ctx.process_actor_pending(index)?;
          let has_pending = ctx.actor_has_pending(index);
          ctx.mark_idle(index, has_pending);
          if processed {
            return Ok(());
          }
          continue;
        }

        if ctx.drain_ready()? {
          return Ok(());
        }
      }

      let wait_future_opt = {
        let ctx = self.context.lock();
        ctx.wait_for_any_signal_future()
      };

      let Some(wait_future) = wait_future_opt else {
        return Ok(());
      };
      let index = wait_future.await;

      let ctx = self.context.lock();
      ctx.enqueue_ready(index);
    }
  }
}
#[async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, R> for PriorityScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    PriorityScheduler::spawn_actor(self, supervisor, context)
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) {
    PriorityScheduler::set_receive_timeout_factory(self, factory);
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    PriorityScheduler::set_root_event_listener(self, listener);
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    PriorityScheduler::set_root_escalation_handler(self, handler);
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    PriorityScheduler::set_metrics_sink(self, sink);
  }

  fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    PriorityScheduler::set_parent_guardian(self, control_ref, map_system);
  }

  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    PriorityScheduler::set_root_failure_telemetry(self, telemetry);
  }

  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    PriorityScheduler::set_root_observation_config(self, config);
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  ) {
    PriorityScheduler::on_escalation(self, handler);
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    PriorityScheduler::take_escalations(self)
  }

  fn actor_count(&self) -> usize {
    PriorityScheduler::actor_count(self)
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    PriorityScheduler::drain_ready(self)
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    PriorityScheduler::dispatch_next(self).await
  }
}

#[async_trait(?Send)]
impl<M, R, Strat> ActorScheduler<M, R> for ReadyQueueScheduler<M, R, Strat>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  Strat: GuardianStrategy<M, R>,
{
  fn spawn_actor(
    &mut self,
    supervisor: Box<dyn Supervisor<M>>,
    context: SchedulerSpawnContext<M, R>,
  ) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>> {
    ReadyQueueScheduler::spawn_actor(self, supervisor, context)
  }

  fn set_receive_timeout_factory(&mut self, factory: Option<ReceiveTimeoutFactoryShared<M, R>>) {
    ReadyQueueScheduler::set_receive_timeout_factory(self, factory)
  }

  fn set_root_event_listener(&mut self, listener: Option<FailureEventListener>) {
    ReadyQueueScheduler::set_root_event_listener(self, listener)
  }

  fn set_root_escalation_handler(&mut self, handler: Option<FailureEventHandler>) {
    ReadyQueueScheduler::set_root_escalation_handler(self, handler)
  }

  fn set_metrics_sink(&mut self, sink: Option<MetricsSinkShared>) {
    ReadyQueueScheduler::set_metrics_sink(self, sink)
  }

  fn set_parent_guardian(&mut self, control_ref: InternalActorRef<M, R>, map_system: MapSystemShared<M>) {
    ReadyQueueScheduler::set_parent_guardian(self, control_ref, map_system)
  }

  fn set_root_failure_telemetry(&mut self, telemetry: FailureTelemetryShared) {
    ReadyQueueScheduler::set_root_failure_telemetry(self, telemetry)
  }

  fn set_root_observation_config(&mut self, config: TelemetryObservationConfig) {
    ReadyQueueScheduler::set_root_observation_config(self, config)
  }

  fn on_escalation(
    &mut self,
    handler: Box<dyn FnMut(&FailureInfo) -> Result<(), QueueError<PriorityEnvelope<M>>> + 'static>,
  ) {
    ReadyQueueScheduler::on_escalation(self, handler)
  }

  fn take_escalations(&mut self) -> Vec<FailureInfo> {
    ReadyQueueScheduler::take_escalations(self)
  }

  fn actor_count(&self) -> usize {
    ReadyQueueScheduler::actor_count(self)
  }

  fn drain_ready(&mut self) -> Result<bool, QueueError<PriorityEnvelope<M>>> {
    ReadyQueueScheduler::drain_ready(self)
  }

  async fn dispatch_next(&mut self) -> Result<(), QueueError<PriorityEnvelope<M>>> {
    ReadyQueueScheduler::dispatch_next(self).await
  }

  fn ready_queue_worker(&self) -> Option<ArcShared<dyn ReadyQueueWorker<M, R>>> {
    Some(self.worker_handle())
  }
}
use crate::ShutdownToken;
