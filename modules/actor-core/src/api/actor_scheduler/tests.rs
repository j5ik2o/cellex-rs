#![allow(deprecated, unused_imports)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::disallowed_types)]

extern crate std;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use core::{
  cell::{Cell, RefCell},
  task::{Context, Poll},
};
use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
};

use cellex_utils_core_rs::{
  collections::{
    queue::{
      backend::{OverflowPolicy, QueueError},
      priority::DEFAULT_PRIORITY,
      QueueSize,
    },
    Element,
  },
  sync::ArcShared,
};
use futures::{
  executor::{block_on, LocalPool},
  future::{poll_fn, FutureExt},
  task::LocalSpawnExt,
};
use spin::RwLock;

use super::{ready_queue_scheduler::ReadyQueueScheduler, *};
use crate::{
  api::{
    actor::{
      actor_context::ActorContext, actor_failure::BehaviorFailure, actor_ref::PriorityActorRef, behavior::Behavior,
      ActorHandlerFn, ActorId, ChildNaming, Props, ShutdownToken, SpawnError,
    },
    actor_runtime::{GenericActorRuntime, MailboxConcurrencyOf},
    actor_scheduler::{
      actor_scheduler_handle_builder::ActorSchedulerHandleBuilder,
      ready_queue_coordinator::{InvokeResult, MailboxIndex, ReadyQueueCoordinator, ResumeCondition, SignalKey},
      ready_queue_scheduler::{drive_ready_queue_worker, ReadyQueueHandle, ReadyQueueWorker},
      ActorScheduler, ActorSchedulerSpawnContext,
    },
    extensions::Extensions,
    failure::{failure_event_stream::FailureEventListener, FailureEvent, FailureInfo},
    guardian::{AlwaysRestart, GuardianStrategy},
    mailbox::{
      messages::{PriorityChannel, SystemMessage},
      queue_mailbox::{QueueMailbox, SyncMailbox, SyncMailboxProducer, SyncMailboxQueue, SystemMailboxQueue},
      Mailbox, ThreadSafe,
    },
    metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
    process::{
      pid::{Pid, SystemId},
      process_registry::ProcessRegistry,
    },
    supervision::supervisor::{NoopSupervisor, Supervisor, SupervisorDirective},
    test_support::{TestMailboxFactory, TestSignal},
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxConsumer, MailboxFactory, MailboxOptions, MailboxPair},
    messaging::{AnyMessage, MapSystemShared, MessageEnvelope},
    supervision::FailureEventHandler,
  },
};

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct AlwaysEscalate;

impl<MF> GuardianStrategy<MF> for AlwaysEscalate
where
  MF: MailboxFactory,
{
  fn decide(&mut self, _actor: ActorId, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    SupervisorDirective::Escalate
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Message {
  User(u32),
  System(SystemMessage),
}

fn dyn_user(value: u32) -> AnyMessage {
  AnyMessage::new(MessageEnvelope::<Message>::user(Message::User(value)))
}

fn dyn_system(message: SystemMessage) -> AnyMessage {
  AnyMessage::new(MessageEnvelope::<Message>::System(message))
}

#[derive(Clone)]
struct EventRecordingSink {
  events: Arc<Mutex<Vec<MetricsEvent>>>,
}

impl EventRecordingSink {
  fn new(events: Arc<Mutex<Vec<MetricsEvent>>>) -> Self {
    Self { events }
  }
}

impl MetricsSink for EventRecordingSink {
  fn record(&self, event: MetricsEvent) {
    self.events.lock().unwrap().push(event);
  }
}

#[derive(Clone, Copy)]
struct SyncMailboxFactory {
  capacity: usize,
  policy:   OverflowPolicy,
}

impl SyncMailboxFactory {
  const fn bounded(capacity: usize, policy: OverflowPolicy) -> Self {
    Self { capacity, policy }
  }

  fn resolve_capacity(&self, options: MailboxOptions) -> usize {
    options.capacity_limit().unwrap_or(self.capacity).max(1)
  }
}

type SchedulerMailbox<M> = SyncMailbox<M, TestSignal>;
type SchedulerMailboxProducer<M> = SyncMailboxProducer<M, TestSignal>;

impl MailboxFactory for SyncMailboxFactory {
  type Concurrency = ThreadSafe;
  type Mailbox<M>
    = SchedulerMailbox<M>
  where
    M: Element;
  type Producer<M>
    = SchedulerMailboxProducer<M>
  where
    M: Element;
  type Queue<M>
    = SystemMailboxQueue<M>
  where
    M: Element;
  type Signal = TestSignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let capacity = self.resolve_capacity(options);
    let base = SyncMailboxQueue::bounded(capacity, self.policy);
    let queue = SystemMailboxQueue::new(base, None);
    let signal = TestSignal::default();
    let mailbox: SchedulerMailbox<M> = QueueMailbox::new(queue, signal);
    let producer: SchedulerMailboxProducer<M> = mailbox.producer();
    (mailbox, producer)
  }
}

type SchedulerTestRuntime<MF> = GenericActorRuntime<MF>;

struct RecordingCoordinator {
  events: Arc<Mutex<Vec<(MailboxIndex, InvokeResult)>>>,
}

impl RecordingCoordinator {
  fn new(events: Arc<Mutex<Vec<(MailboxIndex, InvokeResult)>>>) -> Self {
    Self { events }
  }
}

impl ReadyQueueCoordinator for RecordingCoordinator {
  fn register_ready(&mut self, _idx: MailboxIndex) {}

  fn unregister(&mut self, _idx: MailboxIndex) {}

  fn drain_ready_cycle(&mut self, _max_batch: usize, _out: &mut Vec<MailboxIndex>) {}

  fn poll_wait_signal(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
    Poll::Pending
  }

  fn handle_invoke_result(&mut self, idx: MailboxIndex, result: InvokeResult) {
    self.events.lock().unwrap().push((idx, result));
  }

  fn throughput_hint(&self) -> usize {
    1
  }
}

fn handler_from_fn<M, MF, F>(mut f: F) -> Box<ActorHandlerFn<AnyMessage, MF>>
where
  M: Element,
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  F: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, M, SchedulerTestRuntime<MF>>, MessageEnvelope<M>) + 'static, {
  Box::new(move |ctx, message| {
    let envelope = message.downcast::<MessageEnvelope<M>>().expect("unexpected message type delivered to test handler");
    match envelope {
      | MessageEnvelope::User(user) => {
        let (msg, metadata) = user.into_parts::<MailboxConcurrencyOf<SchedulerTestRuntime<MF>>>();
        let metadata = metadata.unwrap_or_default();
        let mut typed_ctx = ActorContext::with_metadata(ctx, metadata);
        f(&mut typed_ctx, MessageEnvelope::user(msg));
      },
      | MessageEnvelope::System(sys) => {
        let mut typed_ctx = ActorContext::new(ctx);
        f(&mut typed_ctx, MessageEnvelope::System(sys));
      },
    }
    Ok(())
  })
}

fn handler_from_message<MF, F>(mut f: F) -> Box<ActorHandlerFn<AnyMessage, MF>>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  F: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, Message, SchedulerTestRuntime<MF>>, Message) + 'static, {
  handler_from_fn::<Message, MF, _>(move |ctx, envelope| match envelope {
    | MessageEnvelope::User(user) => {
      let (msg, _) = user.into_parts::<MailboxConcurrencyOf<SchedulerTestRuntime<MF>>>();
      f(ctx, msg);
    },
    | MessageEnvelope::System(sys) => {
      f(ctx, Message::System(sys));
    },
  })
}

fn spawn_with_runtime<MF>(
  scheduler: &mut dyn ActorScheduler<MF>,
  mailbox_factory: MF,
  supervisor: Box<dyn Supervisor<AnyMessage>>,
  options: MailboxOptions,
  map_system: MapSystemShared<AnyMessage>,
  handler: Box<ActorHandlerFn<AnyMessage, MF>>,
) -> Result<PriorityActorRef<AnyMessage, MF>, QueueError<PriorityEnvelope<AnyMessage>>>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone, {
  let mailbox_factory_shared = ArcShared::new(mailbox_factory.clone());
  let process_registry = ArcShared::new(ProcessRegistry::new(SystemId::new("test"), None));
  let pid_slot = ArcShared::new(RwLock::new(None::<Pid>));
  let context: ActorSchedulerSpawnContext<MF> = ActorSchedulerSpawnContext {
    mailbox_factory,
    mailbox_factory_shared,
    map_system,
    mailbox_options: options,
    handler,
    child_naming: ChildNaming::Auto,
    process_registry,
    actor_pid_slot: pid_slot,
  };
  scheduler.spawn_actor(supervisor, context).map_err(|err| match err {
    | SpawnError::Queue(queue_err) => queue_err,
    | SpawnError::NameExists(name) => {
      debug_assert!(false, "unexpected name conflict in scheduler test: {name}");
      QueueError::Disconnected
    },
  })
}

#[test]
fn scheduler_delivers_watch_before_user_messages() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let _actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| {
      log_clone.borrow_mut().push(msg);
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::System(SystemMessage::Watch(ActorId::ROOT))]);
}

#[test]
fn scheduler_handle_trait_object_dispatches() {
  use futures::executor::block_on;

  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ActorSchedulerHandleBuilder::ready_queue().build(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  spawn_with_runtime(
    scheduler.as_mut(),
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| {
      log_clone.borrow_mut().push(msg);
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::System(SystemMessage::Watch(ActorId::ROOT))]);
}

#[test]
fn scheduler_builder_dispatches() {
  use futures::executor::block_on;

  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ActorSchedulerHandleBuilder::ready_queue().build(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  spawn_with_runtime(
    scheduler.as_mut(),
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| {
      log_clone.borrow_mut().push(msg);
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::System(SystemMessage::Watch(ActorId::ROOT))]);
}

#[test]
fn priority_scheduler_emits_actor_lifecycle_metrics() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());
  let events = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_metrics_sink(Some(MetricsSinkShared::new(EventRecordingSink::new(events.clone()))));

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message::<TestMailboxFactory, _>(|_, _| {}),
  )
  .unwrap();

  {
    let recorded = events.lock().unwrap();
    let registered = recorded.iter().filter(|event| matches!(event, MetricsEvent::ActorRegistered)).count();
    let deregistered = recorded.iter().filter(|event| matches!(event, MetricsEvent::ActorDeregistered)).count();
    let enqueued = recorded.iter().filter(|event| matches!(event, MetricsEvent::MailboxEnqueued)).count();
    let dequeued = recorded.iter().filter(|event| matches!(event, MetricsEvent::MailboxDequeued)).count();
    assert_eq!(registered, 1);
    assert_eq!(deregistered, 0);
    assert!(enqueued >= 1, "expected at least one MailboxEnqueued event, got {recorded:?}");
    assert_eq!(dequeued, 0);
  }

  actor_ref.sender().try_send(PriorityEnvelope::from_system(SystemMessage::Stop).map(dyn_system)).unwrap();
  block_on(scheduler.dispatch_next()).unwrap();
  scheduler.drain_ready().unwrap();

  {
    let recorded = events.lock().unwrap();
    let registered = recorded.iter().filter(|event| matches!(event, MetricsEvent::ActorRegistered)).count();
    let deregistered = recorded.iter().filter(|event| matches!(event, MetricsEvent::ActorDeregistered)).count();
    let dequeued = recorded.iter().filter(|event| matches!(event, MetricsEvent::MailboxDequeued)).count();
    let enqueued = recorded.iter().filter(|event| matches!(event, MetricsEvent::MailboxEnqueued)).count();
    assert_eq!(registered, 1);
    assert_eq!(deregistered, 1);
    assert!(dequeued >= 1, "expected at least one MailboxDequeued event, got {recorded:?}");
    assert!(enqueued >= dequeued);
  }
}

#[test]
fn scheduler_records_drop_oldest_metric() {
  let mailbox_factory = SyncMailboxFactory::bounded(1, OverflowPolicy::DropOldest);
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());
  let events = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_metrics_sink(Some(MetricsSinkShared::new(EventRecordingSink::new(events.clone()))));

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message::<SyncMailboxFactory, _>(|_, _| {}),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  actor_ref.try_send_with_priority(dyn_user(1), DEFAULT_PRIORITY).unwrap();
  actor_ref.try_send_with_priority(dyn_user(2), DEFAULT_PRIORITY).unwrap();

  let recorded = events.lock().unwrap().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxDroppedOldest { count } if *count == 1)),
    "expected MailboxDroppedOldest event, got {recorded:?}"
  );
}

#[test]
fn scheduler_records_drop_newest_metric() {
  let mailbox_factory = SyncMailboxFactory::bounded(1, OverflowPolicy::DropNewest);
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());
  let events = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_metrics_sink(Some(MetricsSinkShared::new(EventRecordingSink::new(events.clone()))));

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message::<SyncMailboxFactory, _>(|_, _| {}),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  actor_ref.try_send_with_priority(dyn_user(1), DEFAULT_PRIORITY).unwrap();
  let Err(err) = actor_ref.try_send_with_priority(dyn_user(2), DEFAULT_PRIORITY) else {
    panic!("second send should fail when dropping newest");
  };
  assert!(matches!(err, QueueError::Full(_)));

  let recorded = events.lock().unwrap().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxDroppedNewest { count } if *count == 1)),
    "expected MailboxDroppedNewest event, got {recorded:?}"
  );
}

#[test]
fn actor_context_exposes_parent_watcher() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let watchers_log: Rc<RefCell<Vec<Vec<ActorId>>>> = Rc::new(RefCell::new(Vec::new()));
  let watchers_clone = watchers_log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |ctx, msg| {
      let current_watchers = ctx.watchers().to_vec();
      watchers_clone.borrow_mut().push(current_watchers);
      match msg {
        | Message::User(_) | Message::System(_) => {},
      }
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();
  assert_eq!(watchers_log.borrow().as_slice(), &[vec![ActorId::ROOT]]);

  actor_ref.try_send_with_priority(dyn_user(1), DEFAULT_PRIORITY).unwrap();
  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(watchers_log.borrow().as_slice(), &[vec![ActorId::ROOT], vec![ActorId::ROOT]]);
}

#[test]
fn scheduler_dispatches_high_priority_first() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<(u32, i8)>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message::<TestMailboxFactory, _>(move |ctx, msg| match msg {
      | Message::User(value) => {
        log_clone.borrow_mut().push((value, ctx.current_priority().unwrap()));
        if value == 99 {
          let child_log_outer = log_clone.clone();
          let child_props = Props::<Message, SchedulerTestRuntime<TestMailboxFactory>>::with_behavior({
            let child_log = child_log_outer;
            move || {
              let child_log = child_log.clone();
              Behavior::stateless(move |_child_ctx, child_msg: Message| {
                if let Message::User(child_value) = child_msg {
                  child_log.borrow_mut().push((child_value, 0));
                }
                Ok(())
              })
            }
          });
          ctx.spawn_child(child_props).tell_with_priority(Message::User(7), 0).unwrap();
        }
      },
      | Message::System(_) => {},
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(10), 1).unwrap();
  actor_ref.try_send_with_priority(dyn_user(99), 7).unwrap();
  actor_ref.try_send_with_priority(dyn_user(20), 3).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();
  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(scheduler.actor_count(), 2);

  assert_eq!(log.borrow().as_slice(), &[(99, 7), (20, 3), (10, 1), (7, 0)]);
}

#[test]
fn scheduler_prioritizes_system_messages() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| {
      log_clone.borrow_mut().push(msg);
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(42), DEFAULT_PRIORITY).unwrap();

  let control_envelope = PriorityEnvelope::from_system(SystemMessage::Stop).map(dyn_system);
  actor_ref.try_send_envelope(control_envelope).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::System(SystemMessage::Stop)]);
}

#[test]
fn scheduler_reports_suspend_result_to_coordinator() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let events: Arc<Mutex<Vec<(MailboxIndex, InvokeResult)>>> = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_ready_queue_coordinator(Some(Box::new(RecordingCoordinator::new(events.clone()))));

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| {
      if let Message::User(value) = msg {
        log_clone.borrow_mut().push(Message::User(value));
      }
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  let suspend_envelope = PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system);
  actor_ref.try_send_envelope(suspend_envelope).expect("send suspend");
  actor_ref.try_send_with_priority(dyn_user(7), DEFAULT_PRIORITY).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  let resume_key = {
    let recordings = events.lock().unwrap();
    recordings
      .iter()
      .rev()
      .find_map(|(_, result)| {
        if let InvokeResult::Suspended { resume_on, .. } = result {
          if let ResumeCondition::ExternalSignal(key) = resume_on {
            return Some(*key);
          }
        }
        None
      })
      .expect("suspend result should be recorded")
  };

  assert!(log.borrow().is_empty(), "suspended actor must not process user messages");

  scheduler.notify_resume_signal(resume_key);

  block_on(scheduler.dispatch_next()).unwrap();
  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::User(7)]);
}

#[test]
fn priority_actor_ref_sends_system_messages() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler = ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<SystemMessage>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| {
      if let Message::System(system) = msg {
        log_clone.borrow_mut().push(system);
      }
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_system(SystemMessage::Restart), DEFAULT_PRIORITY).unwrap();
  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[SystemMessage::Watch(ActorId::ROOT), SystemMessage::Restart]);
}

#[cfg(feature = "unwind-supervision")]
#[test]
fn scheduler_notifies_guardian_and_restarts_on_panic() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();
  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| {
      match msg {
        | Message::System(SystemMessage::Watch(_)) => {
          // Watch メッセージは監視登録のみなのでログに残さない
        },
        | Message::User(_) if panic_flag.get() => {
          panic_flag.set(false);
          panic!("boom");
        },
        | _ => {
          log_clone.borrow_mut().push(msg.clone());
        },
      }
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(1), DEFAULT_PRIORITY).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();
  assert!(log.borrow().is_empty());

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::System(SystemMessage::Restart)]);
  assert!(!should_panic.get());
}

#[test]
fn scheduler_run_until_processes_messages() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| match msg {
      | Message::User(value) => log_clone.borrow_mut().push(Message::User(value)),
      | Message::System(_) => {},
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(11), DEFAULT_PRIORITY).unwrap();

  let mut loops = 0;
  futures::executor::block_on(scheduler.run_until(|| {
    let continue_loop = loops == 0;
    loops += 1;
    continue_loop
  }))
  .unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::User(11)]);
}

#[cfg(feature = "unwind-supervision")]
#[test]
fn scheduler_records_escalations() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysEscalate> =
    ReadyQueueScheduler::with_strategy(mailbox_factory.clone(), AlwaysEscalate, Extensions::new());

  let sink: Rc<RefCell<Vec<FailureInfo>>> = Rc::new(RefCell::new(Vec::new()));
  let sink_clone = sink.clone();
  scheduler.on_escalation(move |info| {
    sink_clone.borrow_mut().push(info.clone());
    Ok(())
  });

  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| match msg {
      | Message::System(SystemMessage::Watch(_)) => {},
      | Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("boom");
      },
      | _ => {},
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(1), DEFAULT_PRIORITY).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  let handler_data = sink.borrow();
  assert_eq!(handler_data.len(), 1);
  assert_eq!(handler_data[0].actor, ActorId(0));
  let description = handler_data[0].description();
  assert!(description.starts_with("panic:"));

  // handler で除去済みのため take_escalations は空
  assert!(scheduler.take_escalations().is_empty());
}

#[cfg(feature = "unwind-supervision")]
#[test]
fn scheduler_escalation_handler_delivers_to_parent() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysEscalate> =
    ReadyQueueScheduler::with_strategy(mailbox_factory.clone(), AlwaysEscalate, Extensions::new());

  let (parent_mailbox, parent_sender) = mailbox_factory.build_default_mailbox::<PriorityEnvelope<AnyMessage>>();
  let parent_ref: PriorityActorRef<AnyMessage, TestMailboxFactory> = PriorityActorRef::new(parent_sender);
  scheduler.set_parent_guardian(parent_ref.clone(), MapSystemShared::new(dyn_system));

  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| match msg {
      | Message::System(SystemMessage::Watch(_)) => {},
      | Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("boom");
      },
      | _ => {},
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(1), DEFAULT_PRIORITY).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  let envelope = parent_mailbox.try_dequeue().unwrap().unwrap();
  let (msg, _, channel) = envelope.into_parts_with_channel();
  assert_eq!(channel, PriorityChannel::Control);
  match msg.downcast::<MessageEnvelope<Message>>().expect("expected MessageEnvelope<Message> in parent mailbox") {
    | MessageEnvelope::System(SystemMessage::Escalate(info)) => {
      assert_eq!(info.actor, ActorId(0));
      assert!(info.description().contains("panic"));
    },
    | other => panic!("unexpected message: {:?}", other),
  }
}

#[cfg(feature = "unwind-supervision")]
#[test]
fn scheduler_escalation_chain_reaches_root() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysEscalate> =
    ReadyQueueScheduler::with_strategy(mailbox_factory.clone(), AlwaysEscalate, Extensions::new());

  let collected: Rc<RefCell<Vec<FailureInfo>>> = Rc::new(RefCell::new(Vec::new()));
  let collected_clone = collected.clone();
  scheduler.on_escalation(move |info| {
    collected_clone.borrow_mut().push(info.clone());
    Ok(())
  });

  let parent_triggered = Rc::new(Cell::new(false));
  let trigger_flag = parent_triggered.clone();
  let child_panics = Rc::new(Cell::new(true));
  let child_flag = child_panics.clone();

  let parent_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message::<TestMailboxFactory, _>(move |ctx, msg| match msg {
      | Message::System(_) => {},
      | Message::User(0) if !trigger_flag.get() => {
        trigger_flag.set(true);
        let panic_once = child_flag.clone();
        let child_props = Props::<Message, SchedulerTestRuntime<TestMailboxFactory>>::with_behavior({
          let panic_once = panic_once.clone();
          move || {
            let panic_once = panic_once.clone();
            Behavior::stateless(move |_child_ctx, child_msg: Message| {
              match child_msg {
                | Message::System(_) => {},
                | Message::User(1) if panic_once.get() => {
                  panic_once.set(false);
                  panic!("child failure");
                },
                | _ => {},
              }
              Ok(())
            })
          }
        });
        ctx.spawn_child(child_props).tell_with_priority(Message::User(1), DEFAULT_PRIORITY).unwrap();
      },
      | _ => {},
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  {
    let snapshot = collected.borrow();
    assert_eq!(snapshot.len(), 0);
  }

  parent_ref.try_send_with_priority(dyn_user(0), DEFAULT_PRIORITY).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();
  {
    let snapshot = collected.borrow();
    assert_eq!(snapshot.len(), 0);
  }

  block_on(scheduler.dispatch_next()).unwrap();
  {
    let snapshot = collected.borrow();
    assert_eq!(snapshot.len(), 1);
  }
  let first_failure = collected.borrow()[0].clone();
  let first_stage = first_failure.failure_escalation_stage;
  assert!(first_stage.hops() >= 1, "child escalation should have hop count >= 1");

  let parent_failure = first_failure.escalate_to_parent().expect("parent failure info must exist");
  let parent_stage = parent_failure.failure_escalation_stage;
  assert!(parent_stage.hops() >= first_stage.hops(), "parent escalation hop count must be monotonic");

  let mut current = parent_failure.clone();
  let mut root_failure = current.clone();
  while let Some(next) = current.escalate_to_parent() {
    root_failure = next.clone();
    current = next;
  }
  let root_stage = root_failure.failure_escalation_stage;

  assert_eq!(first_failure.path.segments().last().copied(), Some(first_failure.actor));

  assert_eq!(parent_failure.actor, first_failure.path.segments().first().copied().unwrap_or(first_failure.actor));

  assert_eq!(root_failure.actor, parent_failure.actor);
  assert!(root_failure.path.is_empty());
  assert_eq!(root_failure.description(), parent_failure.description());
  assert!(root_stage.hops() >= parent_stage.hops(), "root escalation hop count must be monotonic");
}

#[cfg(feature = "unwind-supervision")]
#[test]
fn scheduler_root_escalation_handler_invoked() {
  use std::sync::{Arc as StdArc, Mutex};

  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysEscalate> =
    ReadyQueueScheduler::with_strategy(mailbox_factory.clone(), AlwaysEscalate, Extensions::new());

  let events: StdArc<Mutex<Vec<FailureInfo>>> = StdArc::new(Mutex::new(Vec::new()));
  let events_clone = events.clone();

  scheduler.set_root_escalation_handler(Some(FailureEventHandler::new(move |info: &FailureInfo| {
    events_clone.lock().unwrap().push(info.clone());
  })));

  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| match msg {
      | Message::System(SystemMessage::Watch(_)) => {},
      | Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("root boom");
      },
      | _ => {},
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(42), DEFAULT_PRIORITY).unwrap();

  let events_ref = events.clone();
  block_on(scheduler.run_until(|| events_ref.lock().unwrap().is_empty())).unwrap();

  let events = events.lock().unwrap();
  assert_eq!(events.len(), 1);
  assert!(!events[0].description().is_empty());
}

#[cfg(feature = "unwind-supervision")]
#[test]
fn scheduler_requeues_failed_custom_escalation() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysEscalate> =
    ReadyQueueScheduler::with_strategy(mailbox_factory.clone(), AlwaysEscalate, Extensions::new());

  let attempts = Rc::new(Cell::new(0usize));
  let attempts_clone = attempts.clone();
  scheduler.on_escalation(move |info| {
    assert!(
      info.failure_escalation_stage.hops() >= 1,
      "escalation delivered to custom sink should already have propagated"
    );
    let current = attempts_clone.get();
    attempts_clone.set(current + 1);
    if current == 0 {
      Err(QueueError::Disconnected)
    } else {
      Ok(())
    }
  });

  let panic_flag = Rc::new(Cell::new(true));
  let panic_once = panic_flag.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| match msg {
      | Message::System(_) => {},
      | Message::User(_) if panic_once.get() => {
        panic_once.set(false);
        panic!("custom escalation failure");
      },
      | _ => {},
    }),
  )
  .unwrap();

  // consume initial watch message
  block_on(scheduler.dispatch_next()).unwrap();

  actor_ref.try_send_with_priority(dyn_user(7), DEFAULT_PRIORITY).unwrap();

  // first dispatch: panic occurs and escalation handler fails, causing requeue.
  block_on(scheduler.dispatch_next()).unwrap();
  assert_eq!(attempts.get(), 1);

  // second dispatch: retry succeeds and escalation queue drains.
  block_on(scheduler.dispatch_next()).unwrap();
  assert_eq!(attempts.get(), 2);
  assert!(scheduler.take_escalations().is_empty());
}

#[cfg(all(feature = "unwind-supervision", feature = "test-support"))]
#[test]
fn scheduler_root_event_listener_broadcasts() {
  use std::sync::{Arc as StdArc, Mutex};

  use crate::api::failure::failure_event_stream::{tests::TestFailureEventStream, FailureEventStream};

  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysEscalate> =
    ReadyQueueScheduler::with_strategy(mailbox_factory.clone(), AlwaysEscalate, Extensions::new());

  let hub = TestFailureEventStream::default();
  let received: StdArc<Mutex<Vec<FailureInfo>>> = StdArc::new(Mutex::new(Vec::new()));
  let received_clone = received.clone();

  let _subscription = hub.subscribe(FailureEventListener::new(move |event| match event {
    | crate::api::failure::FailureEvent::RootEscalated(info) => {
      received_clone.lock().unwrap().push(info.clone());
    },
  }));

  scheduler.set_root_event_listener(Some(hub.listener()));

  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    mailbox_factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(dyn_system),
    handler_from_message(move |_, msg| match msg {
      | Message::System(SystemMessage::Watch(_)) => {},
      | Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("hub boom");
      },
      | _ => {},
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(dyn_user(7), DEFAULT_PRIORITY).unwrap();

  let received_ref = received.clone();
  block_on(scheduler.run_until(|| received_ref.lock().unwrap().is_empty())).unwrap();

  let events = received.lock().unwrap();
  assert_eq!(events.len(), 1);
  assert!(!events[0].description().is_empty());
}

#[test]
fn drive_ready_queue_worker_processes_actions() {
  use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
  };

  use futures::future::LocalBoxFuture;

  struct YieldOnce {
    yielded: bool,
  }

  impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
      if self.yielded {
        Poll::Ready(())
      } else {
        self.yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
      }
    }
  }

  type WorkerState = (VecDeque<WorkerAction>, Option<LocalBoxFuture<'static, usize>>, bool);

  #[allow(clippy::arc_with_non_send_sync)]
  struct DummyWorker {
    state:     Arc<Mutex<WorkerState>>,
    processed: Arc<Mutex<Vec<u32>>>,
  }

  #[derive(Clone)]
  enum WorkerAction {
    Progress(u32),
    Wait,
    End,
  }

  impl DummyWorker {
    #[allow(clippy::arc_with_non_send_sync)]
    fn new(actions: VecDeque<WorkerAction>, processed: Arc<Mutex<Vec<u32>>>) -> Self {
      Self { state: Arc::new(Mutex::new((actions, None, false))), processed }
    }
  }

  impl ReadyQueueWorker<TestMailboxFactory> for DummyWorker {
    fn process_ready_once(&self) -> Result<Option<bool>, QueueError<PriorityEnvelope<AnyMessage>>> {
      let mut state = self.state.lock().unwrap();
      let (actions, wait_future, finished) = &mut *state;
      if let Some(action) = actions.pop_front() {
        match action {
          | WorkerAction::Progress(value) => {
            self.processed.lock().unwrap().push(value);
            Ok(Some(true))
          },
          | WorkerAction::Wait => {
            *wait_future = Some(futures::future::ready(0usize).boxed_local());
            Ok(None)
          },
          | WorkerAction::End => {
            *finished = true;
            Ok(None)
          },
        }
      } else {
        Ok(None)
      }
    }

    fn wait_for_ready(&self) -> Option<LocalBoxFuture<'static, usize>> {
      let mut state = self.state.lock().unwrap();
      let (_, wait_future, _finished) = &mut *state;
      wait_future.take()
    }
  }

  fn shutdown_poll_future(token: ShutdownToken) -> impl core::future::Future<Output = ()> {
    poll_fn(move |cx| {
      if token.is_triggered() {
        core::task::Poll::Ready(())
      } else {
        cx.waker().wake_by_ref();
        core::task::Poll::Pending
      }
    })
  }

  let processed = Arc::new(Mutex::new(Vec::new()));
  let actions =
    VecDeque::from(vec![WorkerAction::Progress(1), WorkerAction::Wait, WorkerAction::Progress(2), WorkerAction::End]);
  let worker_impl = DummyWorker::new(actions, processed.clone());
  let worker = ArcShared::new(worker_impl).into_dyn(|inner| inner as &dyn ReadyQueueWorker<TestMailboxFactory>);

  let shutdown = ShutdownToken::default();
  let shutdown_for_worker = shutdown.clone();
  let shutdown_for_wait = shutdown.clone();

  let mut pool = LocalPool::new();
  pool
    .spawner()
    .spawn_local(
      drive_ready_queue_worker(
        worker,
        shutdown_for_worker,
        || {
          // LocalPool 上で他タスクに制御を明示的に渡すため即時完了 Future ではなく 1 回だけ Pending になる
          // Future を使う
          YieldOnce { yielded: false }
        },
        move || shutdown_poll_future(shutdown_for_wait.clone()),
      )
      .map(|res| res.expect("worker loop succeeds")),
    )
    .expect("spawn worker loop");

  let shutdown_trigger = shutdown;
  let processed_observer = processed.clone();
  pool
    .spawner()
    .spawn_local(async move {
      poll_fn(|cx| {
        if processed_observer.lock().unwrap().len() >= 2 {
          core::task::Poll::Ready(())
        } else {
          cx.waker().wake_by_ref();
          core::task::Poll::Pending
        }
      })
      .await;
      shutdown_trigger.trigger();
    })
    .expect("spawn shutdown trigger");

  pool.run();

  let guard = processed.lock().unwrap();
  assert_eq!(&*guard, &[1, 2]);
}
