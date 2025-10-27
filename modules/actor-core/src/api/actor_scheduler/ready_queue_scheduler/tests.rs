#![allow(clippy::unwrap_used)]

extern crate std;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use core::{
  task::{Context, Poll},
  time::Duration,
};
use std::sync::{Arc, Mutex};

use cellex_utils_core_rs::{
  collections::queue::{backend::QueueError, priority::DEFAULT_PRIORITY},
  sync::ArcShared,
};
use spin::RwLock;

use super::*;
use crate::{
  api::{
    actor::{actor_context::ActorContext, actor_ref::PriorityActorRef, ActorHandlerFn, ChildNaming, SpawnError},
    actor_runtime::{GenericActorRuntime, MailboxConcurrencyOf},
    actor_scheduler::{
      ready_queue_coordinator::{InvokeResult, MailboxIndex, ReadyQueueCoordinator, ResumeCondition, SuspendReason},
      ActorScheduler, ActorSchedulerSpawnContext,
    },
    extensions::Extensions,
    guardian::AlwaysRestart,
    mailbox::messages::SystemMessage,
    metrics::{MetricsEvent, MetricsSink, MetricsSinkShared, SuspensionClock, SuspensionClockShared},
    process::{pid::SystemId, process_registry::ProcessRegistry},
    supervision::supervisor::{NoopSupervisor, Supervisor},
    test_support::TestMailboxFactory,
  },
  shared::{
    mailbox::{messages::PriorityEnvelope, MailboxFactory, MailboxOptions},
    messaging::{AnyMessage, MapSystemShared, MessageEnvelope},
  },
};

#[derive(Clone)]
struct MockSuspensionClock {
  now: Arc<Mutex<u64>>,
}

impl MockSuspensionClock {
  fn new(start: u64) -> Self {
    Self { now: Arc::new(Mutex::new(start)) }
  }

  fn advance(&self, delta: u64) {
    let mut guard = self.now.lock().unwrap();
    *guard = guard.saturating_add(delta);
  }
}

impl SuspensionClock for MockSuspensionClock {
  fn now(&self) -> Option<u64> {
    Some(*self.now.lock().unwrap())
  }
}

#[derive(Clone)]
struct RecordingMetricsSink {
  events: Arc<Mutex<Vec<MetricsEvent>>>,
}

impl RecordingMetricsSink {
  fn new(events: Arc<Mutex<Vec<MetricsEvent>>>) -> Self {
    Self { events }
  }
}

impl MetricsSink for RecordingMetricsSink {
  fn record(&self, event: MetricsEvent) {
    self.events.lock().unwrap().push(event);
  }
}

#[derive(Clone)]
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

#[test]
fn resume_condition_after_triggers_on_deadline() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let clock = MockSuspensionClock::new(0);
  scheduler.set_suspension_clock(SuspensionClockShared::new(clock.clone()));

  let log: Rc<core::cell::RefCell<Vec<u32>>> = Rc::new(core::cell::RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_actor(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    handler_from_message(move |_, msg| {
      if let Message::User(value) = msg {
        log_clone.borrow_mut().push(value);
      }
    }),
  )
  .expect("spawn actor");

  let _ = scheduler.drain_ready().unwrap();

  let suspend_envelope = PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system);
  actor_ref.try_send_envelope(suspend_envelope).expect("send suspend");
  let _ = scheduler.drain_ready().unwrap();

  {
    let context = scheduler.context_for_testing();
    let mut ctx = context.lock();
    ctx.core.inject_invoke_result_for_testing(0, InvokeResult::Suspended {
      reason:    SuspendReason::Backpressure,
      resume_on: ResumeCondition::After(Duration::from_nanos(10)),
    });
  }

  actor_ref.try_send_with_priority(dyn_user(5), DEFAULT_PRIORITY).unwrap();

  clock.advance(5);
  assert!(log.borrow().is_empty());

  clock.advance(10);
  for _ in 0..5 {
    let _ = scheduler.drain_ready().unwrap();
    if !log.borrow().is_empty() {
      break;
    }
  }
  assert_eq!(log.borrow().as_slice(), &[5]);
}

#[test]
fn resume_condition_capacity_resumes_immediately() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<core::cell::RefCell<Vec<u32>>> = Rc::new(core::cell::RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_actor(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    handler_from_message(move |_, msg| {
      if let Message::User(value) = msg {
        log_clone.borrow_mut().push(value);
      }
    }),
  )
  .expect("spawn actor");

  let _ = scheduler.drain_ready().unwrap();

  {
    let context = scheduler.context_for_testing();
    let mut ctx = context.lock();
    if let Some(cell) = ctx.core.actor_mut(0) {
      cell.set_scheduler_hook(None);
    }
  }

  let suspend_envelope = PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system);
  actor_ref.try_send_envelope(suspend_envelope).expect("send suspend");
  actor_ref.try_send_with_priority(dyn_user(9), DEFAULT_PRIORITY).unwrap();

  assert!(scheduler.drain_ready().unwrap());
  for _ in 0..5 {
    let _ = scheduler.drain_ready().unwrap();
    if !log.borrow().is_empty() {
      break;
    }
  }
  assert_eq!(log.borrow().as_slice(), &[9]);
}

#[test]
fn metrics_capture_suspend_resume_durations_with_clock() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let clock = MockSuspensionClock::new(0);
  scheduler.set_suspension_clock(SuspensionClockShared::new(clock.clone()));

  let events: Arc<Mutex<Vec<MetricsEvent>>> = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_metrics_sink(Some(MetricsSinkShared::new(RecordingMetricsSink::new(events.clone()))));

  let actor_ref =
    spawn_actor(&mut scheduler, mailbox_factory, Box::new(NoopSupervisor), handler_from_message(|_, _| {}))
      .expect("spawn actor");

  let _ = scheduler.drain_ready().unwrap();

  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system))
    .expect("send suspend-1");
  for _ in 0..3 {
    let _ = scheduler.drain_ready().unwrap();
  }

  clock.advance(10);
  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Resume).map(dyn_system))
    .expect("send resume-1");
  for _ in 0..3 {
    let _ = scheduler.drain_ready().unwrap();
  }

  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system))
    .expect("send suspend-2");
  for _ in 0..3 {
    let _ = scheduler.drain_ready().unwrap();
  }

  clock.advance(20);
  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Resume).map(dyn_system))
    .expect("send resume-2");
  for _ in 0..3 {
    let _ = scheduler.drain_ready().unwrap();
  }

  let recorded = events.lock().unwrap().clone();
  let suspended: Vec<_> = recorded
    .iter()
    .filter_map(|event| {
      if let MetricsEvent::MailboxSuspended { suspend_count, last_duration, total_duration } = event {
        Some((*suspend_count, *last_duration, *total_duration))
      } else {
        None
      }
    })
    .collect();
  let resumed: Vec<_> = recorded
    .iter()
    .filter_map(|event| {
      if let MetricsEvent::MailboxResumed { resume_count, last_duration, total_duration } = event {
        Some((*resume_count, *last_duration, *total_duration))
      } else {
        None
      }
    })
    .collect();

  assert!(suspended.len() >= 2, "unexpected suspended events: {:?}", suspended);
  assert!(resumed.len() >= 2, "unexpected resumed events: {:?}", resumed);

  let second_suspend = suspended[1];
  assert_eq!(second_suspend.0, 2);
  assert_eq!(second_suspend.1, Some(Duration::from_nanos(10)));
  assert_eq!(second_suspend.2, Some(Duration::from_nanos(10)));

  let first_resume = resumed[0];
  assert_eq!(first_resume.0, 1);
  assert_eq!(first_resume.1, Some(Duration::from_nanos(10)));
  assert_eq!(first_resume.2, Some(Duration::from_nanos(10)));

  let second_resume = resumed[1];
  assert_eq!(second_resume.0, 2);
  assert_eq!(second_resume.1, Some(Duration::from_nanos(20)));
  assert_eq!(second_resume.2, Some(Duration::from_nanos(30)));
}

#[test]
fn metrics_omit_duration_when_clock_absent() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  scheduler.set_suspension_clock(SuspensionClockShared::null());

  let events: Arc<Mutex<Vec<MetricsEvent>>> = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_metrics_sink(Some(MetricsSinkShared::new(RecordingMetricsSink::new(events.clone()))));

  let actor_ref =
    spawn_actor(&mut scheduler, mailbox_factory, Box::new(NoopSupervisor), handler_from_message(|_, _| {}))
      .expect("spawn actor");

  let _ = scheduler.drain_ready().unwrap();

  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system))
    .expect("send suspend");
  for _ in 0..3 {
    let _ = scheduler.drain_ready().unwrap();
  }

  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Resume).map(dyn_system))
    .expect("send resume");
  for _ in 0..3 {
    let _ = scheduler.drain_ready().unwrap();
  }

  let recorded = events.lock().unwrap().clone();
  let suspended = recorded.iter().find_map(|event| {
    if let MetricsEvent::MailboxSuspended { suspend_count, last_duration, total_duration } = event {
      Some((*suspend_count, *last_duration, *total_duration))
    } else {
      None
    }
  });
  let resumed = recorded.iter().find_map(|event| {
    if let MetricsEvent::MailboxResumed { resume_count, last_duration, total_duration } = event {
      Some((*resume_count, *last_duration, *total_duration))
    } else {
      None
    }
  });

  let suspended = suspended.expect("missing MailboxSuspended event");
  assert_eq!(suspended.0, 1);
  assert!(suspended.1.is_none());
  assert!(suspended.2.is_none());

  let resumed = resumed.expect("missing MailboxResumed event");
  assert_eq!(resumed.0, 1);
  assert!(resumed.1.is_none());
  assert!(resumed.2.is_none());
}

#[test]
fn multi_actor_suspend_resume_independent() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let recorded: Arc<Mutex<Vec<(MailboxIndex, InvokeResult)>>> = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_ready_queue_coordinator(Some(Box::new(RecordingCoordinator::new(recorded.clone()))));

  let log_a: Rc<core::cell::RefCell<Vec<u32>>> = Rc::new(core::cell::RefCell::new(Vec::new()));
  let log_b: Rc<core::cell::RefCell<Vec<u32>>> = Rc::new(core::cell::RefCell::new(Vec::new()));

  let actor_a = spawn_actor(
    &mut scheduler,
    mailbox_factory.clone(),
    Box::new(NoopSupervisor),
    handler_from_message({
      let log = log_a.clone();
      move |_, msg| {
        if let Message::User(value) = msg {
          log.borrow_mut().push(value);
        }
      }
    }),
  )
  .expect("spawn actor A");

  let actor_b = spawn_actor(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    handler_from_message({
      let log = log_b.clone();
      move |_, msg| {
        if let Message::User(value) = msg {
          log.borrow_mut().push(value);
        }
      }
    }),
  )
  .expect("spawn actor B");

  let _ = scheduler.drain_ready().unwrap();

  {
    let context = scheduler.context_for_testing();
    let mut ctx = context.lock();
    if let Some(cell) = ctx.core.actor_mut(0) {
      cell.set_scheduler_hook(None);
    }
  }

  actor_a
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system))
    .expect("suspend actor A");
  actor_b
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system))
    .expect("suspend actor B");

  actor_a.try_send_with_priority(dyn_user(11), DEFAULT_PRIORITY).unwrap();
  actor_b.try_send_with_priority(dyn_user(22), DEFAULT_PRIORITY).unwrap();

  for _ in 0..6 {
    let _ = scheduler.drain_ready().unwrap();
  }

  assert_eq!(log_a.borrow().as_slice(), &[11]);
  assert!(log_b.borrow().is_empty(), "actor B must remain suspended until signal");

  let resume_key = {
    let events = recorded.lock().unwrap();
    let mut condition_a = None;
    let mut key_b = None;
    for (index, result) in events.iter() {
      if let InvokeResult::Suspended { resume_on, .. } = result {
        if index.slot == 0 {
          condition_a = Some(resume_on.clone());
        } else if index.slot == 1 {
          if let ResumeCondition::ExternalSignal(key) = resume_on {
            key_b = Some(*key);
          }
        }
      }
    }
    assert!(matches!(condition_a, Some(ResumeCondition::WhenCapacityAvailable)), "actor A should resume on capacity");
    key_b.expect("actor B suspend result missing resume signal key")
  };

  scheduler.notify_resume_signal(resume_key);

  for _ in 0..6 {
    let _ = scheduler.drain_ready().unwrap();
    if !log_b.borrow().is_empty() {
      break;
    }
  }

  assert_eq!(log_a.borrow().as_slice(), &[11]);
  assert_eq!(log_b.borrow().as_slice(), &[22]);
}

#[test]
fn backpressure_resumes_pending_messages() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let mut scheduler: ReadyQueueScheduler<TestMailboxFactory, AlwaysRestart> =
    ReadyQueueScheduler::new(mailbox_factory.clone(), Extensions::new());

  let log: Rc<core::cell::RefCell<Vec<u32>>> = Rc::new(core::cell::RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_actor(
    &mut scheduler,
    mailbox_factory,
    Box::new(NoopSupervisor),
    handler_from_message(move |_, msg| {
      if let Message::User(value) = msg {
        log_clone.borrow_mut().push(value);
      }
    }),
  )
  .expect("spawn actor");

  let _ = scheduler.drain_ready().unwrap();

  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system))
    .expect("send suspend");

  assert!(scheduler.drain_ready().unwrap());

  {
    let context = scheduler.context_for_testing();
    let mut ctx = context.lock();
    ctx.core.inject_invoke_result_for_testing(0, InvokeResult::Suspended {
      reason:    SuspendReason::Backpressure,
      resume_on: ResumeCondition::WhenCapacityAvailable,
    });
  }

  actor_ref.try_send_with_priority(dyn_user(77), DEFAULT_PRIORITY).unwrap();

  for _ in 0..6 {
    let _ = scheduler.drain_ready().unwrap();
    if !log.borrow().is_empty() {
      break;
    }
  }

  assert_eq!(log.borrow().as_slice(), &[77]);
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
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

fn handler_from_message<MF, F>(mut f: F) -> Box<ActorHandlerFn<AnyMessage, MF>>
where
  MF: MailboxFactory + Clone + 'static,
  MF::Queue<PriorityEnvelope<AnyMessage>>: Clone,
  MF::Signal: Clone,
  F: for<'r, 'ctx> FnMut(&mut ActorContext<'r, 'ctx, Message, GenericActorRuntime<MF>>, Message) + 'static, {
  Box::new(move |ctx, message| {
    let envelope = message.downcast::<MessageEnvelope<Message>>().expect("unexpected message type");
    match envelope {
      | MessageEnvelope::User(user) => {
        let (msg, _) = user.into_parts::<MailboxConcurrencyOf<GenericActorRuntime<MF>>>();
        f(&mut ActorContext::new(ctx), msg);
      },
      | MessageEnvelope::System(sys) => f(&mut ActorContext::new(ctx), Message::System(sys)),
    }
    Ok(())
  })
}

fn spawn_actor(
  scheduler: &mut dyn ActorScheduler<TestMailboxFactory>,
  mailbox_factory: TestMailboxFactory,
  supervisor: Box<dyn Supervisor<AnyMessage>>,
  handler: Box<ActorHandlerFn<AnyMessage, TestMailboxFactory>>,
) -> Result<PriorityActorRef<AnyMessage, TestMailboxFactory>, QueueError<PriorityEnvelope<AnyMessage>>> {
  let mailbox_factory_shared = ArcShared::new(mailbox_factory.clone());
  let process_registry = ArcShared::new(ProcessRegistry::new(SystemId::new("test"), None));
  let pid_slot = ArcShared::new(RwLock::new(None));
  let context = ActorSchedulerSpawnContext {
    mailbox_factory,
    mailbox_factory_shared,
    map_system: MapSystemShared::new(dyn_system),
    mailbox_options: MailboxOptions::default(),
    handler,
    child_naming: ChildNaming::Auto,
    process_registry,
    actor_pid_slot: pid_slot,
  };

  scheduler.spawn_actor(supervisor, context).map_err(|err| match err {
    | SpawnError::Queue(queue_err) => queue_err,
    | SpawnError::NameExists(_) => QueueError::Disconnected,
  })
}
