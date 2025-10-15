#![allow(deprecated, unused_imports)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::disallowed_types)]
use super::priority_scheduler::PriorityScheduler;
use super::*;
use crate::api::actor::MailboxHandleFactoryStub;
use crate::runtime::context::{ActorContext, InternalActorRef};
use crate::runtime::guardian::{AlwaysRestart, GuardianStrategy};
use crate::runtime::mailbox::test_support::TestMailboxRuntime;
use crate::runtime::scheduler::SchedulerSpawnContext;
use crate::ActorHandlerFn;
use crate::ActorId;
use crate::BehaviorFailure;
use crate::Extensions;
use crate::FailureInfo;
use crate::NoopSupervisor;
use crate::ShutdownToken;
#[cfg(feature = "std")]
use crate::SupervisorDirective;
use crate::{DynMessage, MailboxRuntime, MetricsEvent, MetricsSink, MetricsSinkShared, PriorityEnvelope};
use crate::{FailureEventHandler, FailureEventListener, MapSystemShared};
use crate::{MailboxOptions, Supervisor, SystemMessage};
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use cellex_utils_core_rs::sync::ArcShared;
use cellex_utils_core_rs::{Element, QueueError, DEFAULT_PRIORITY};
#[cfg(feature = "std")]
use core::cell::Cell;
use core::cell::RefCell;
#[cfg(feature = "std")]
use futures::executor::block_on;
#[cfg(feature = "std")]
use futures::executor::LocalPool;
#[cfg(feature = "std")]
use futures::future::{poll_fn, FutureExt};
#[cfg(feature = "std")]
use futures::task::LocalSpawnExt;
#[cfg(feature = "std")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "std")]
use std::{collections::VecDeque, sync::MutexGuard};

#[cfg(feature = "std")]
fn handler_from_fn<M, R, F>(mut f: F) -> Box<ActorHandlerFn<M, R>>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone,
  F: for<'ctx> FnMut(&mut ActorContext<'ctx, M, R, dyn Supervisor<M>>, M) + 'static, {
  Box::new(move |ctx, message| {
    f(ctx, message);
    Ok(())
  })
}

#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug)]
struct AlwaysEscalate;

#[cfg(feature = "std")]
impl<M, R> GuardianStrategy<M, R> for AlwaysEscalate
where
  M: Element,
  R: MailboxRuntime,
{
  fn decide(&mut self, _actor: ActorId, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    SupervisorDirective::Escalate
  }
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Message {
  User(u32),
  System(SystemMessage),
}

#[cfg(feature = "std")]
#[derive(Clone)]
struct EventRecordingSink {
  events: Arc<Mutex<Vec<MetricsEvent>>>,
}

#[cfg(feature = "std")]
impl EventRecordingSink {
  fn new(events: Arc<Mutex<Vec<MetricsEvent>>>) -> Self {
    Self { events }
  }
}

#[cfg(feature = "std")]
impl MetricsSink for EventRecordingSink {
  fn record(&self, event: MetricsEvent) {
    self.events.lock().unwrap().push(event);
  }
}

#[cfg(feature = "std")]
fn spawn_with_runtime<M, R>(
  scheduler: &mut dyn ActorScheduler<M, R>,
  runtime: R,
  supervisor: Box<dyn Supervisor<M>>,
  options: MailboxOptions,
  map_system: MapSystemShared<M>,
  handler: Box<ActorHandlerFn<M, R>>,
) -> Result<InternalActorRef<M, R>, QueueError<PriorityEnvelope<M>>>
where
  M: Element,
  R: MailboxRuntime + Clone + 'static,
  R::Queue<PriorityEnvelope<M>>: Clone,
  R::Signal: Clone, {
  let mailbox_handle_factory_stub = MailboxHandleFactoryStub::from_runtime(runtime.clone());
  let context = SchedulerSpawnContext {
    runtime,
    mailbox_handle_factory_stub: mailbox_handle_factory_stub,
    map_system,
    mailbox_options: options,
    handler,
  };
  scheduler.spawn_actor(supervisor, context)
}

#[cfg(feature = "std")]
#[test]
fn scheduler_delivers_watch_before_user_messages() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler = PriorityScheduler::new(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let _actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| {
      log_clone.borrow_mut().push(msg.clone());
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(
    log.borrow().as_slice(),
    &[Message::System(SystemMessage::Watch(ActorId::ROOT))]
  );
}

#[cfg(feature = "std")]
#[test]
fn scheduler_handle_trait_object_dispatches() {
  use futures::executor::block_on;

  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler = super::SchedulerBuilder::priority().build(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  spawn_with_runtime(
    scheduler.as_mut(),
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| {
      log_clone.borrow_mut().push(msg);
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(
    log.borrow().as_slice(),
    &[Message::System(SystemMessage::Watch(ActorId::ROOT))]
  );
}

#[cfg(feature = "std")]
#[test]
fn immediate_scheduler_builder_dispatches() {
  use futures::executor::block_on;

  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler = super::SchedulerBuilder::immediate().build(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  spawn_with_runtime(
    scheduler.as_mut(),
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| {
      log_clone.borrow_mut().push(msg);
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(
    log.borrow().as_slice(),
    &[Message::System(SystemMessage::Watch(ActorId::ROOT))]
  );
}

#[cfg(feature = "std")]
#[test]
fn priority_scheduler_emits_actor_lifecycle_metrics() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler = PriorityScheduler::new(factory.clone(), Extensions::new());
  let events = Arc::new(Mutex::new(Vec::new()));
  scheduler.set_metrics_sink(Some(MetricsSinkShared::new(EventRecordingSink::new(events.clone()))));

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(|sys| DynMessage::new(sys)),
    handler_from_fn(|_, _msg: DynMessage| {}),
  )
  .unwrap();

  {
    let recorded = events.lock().unwrap();
    let registered = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::ActorRegistered))
      .count();
    let deregistered = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::ActorDeregistered))
      .count();
    let enqueued = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::MailboxEnqueued))
      .count();
    let dequeued = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::MailboxDequeued))
      .count();
    assert_eq!(registered, 1);
    assert_eq!(deregistered, 0);
    assert!(
      enqueued >= 1,
      "expected at least one MailboxEnqueued event, got {recorded:?}"
    );
    assert_eq!(dequeued, 0);
  }

  actor_ref
    .sender()
    .try_send(PriorityEnvelope::from_system(SystemMessage::Stop).map(|sys| DynMessage::new(sys)))
    .unwrap();
  block_on(scheduler.dispatch_next()).unwrap();
  scheduler.drain_ready().unwrap();

  {
    let recorded = events.lock().unwrap();
    let registered = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::ActorRegistered))
      .count();
    let deregistered = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::ActorDeregistered))
      .count();
    let dequeued = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::MailboxDequeued))
      .count();
    let enqueued = recorded
      .iter()
      .filter(|event| matches!(event, MetricsEvent::MailboxEnqueued))
      .count();
    assert_eq!(registered, 1);
    assert_eq!(deregistered, 1);
    assert!(
      dequeued >= 1,
      "expected at least one MailboxDequeued event, got {recorded:?}"
    );
    assert!(enqueued >= dequeued);
  }
}

#[cfg(feature = "std")]
#[test]
fn actor_context_exposes_parent_watcher() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler = PriorityScheduler::new(factory.clone(), Extensions::new());

  let watchers_log: Rc<RefCell<Vec<Vec<ActorId>>>> = Rc::new(RefCell::new(Vec::new()));
  let watchers_clone = watchers_log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |ctx, msg: Message| {
      let current_watchers = ctx.watchers().to_vec();
      watchers_clone.borrow_mut().push(current_watchers);
      match msg {
        Message::User(_) | Message::System(_) => {}
      }
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();
  assert_eq!(watchers_log.borrow().as_slice(), &[vec![ActorId::ROOT]]);

  actor_ref
    .try_send_with_priority(Message::User(1), DEFAULT_PRIORITY)
    .unwrap();
  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(
    watchers_log.borrow().as_slice(),
    &[vec![ActorId::ROOT], vec![ActorId::ROOT]]
  );
}

#[cfg(feature = "std")]
#[test]
fn scheduler_dispatches_high_priority_first() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler = PriorityScheduler::new(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<(u32, i8)>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |ctx, msg: Message| match msg {
      Message::User(value) => {
        log_clone.borrow_mut().push((value, ctx.current_priority().unwrap()));
        if value == 99 {
          let child_log = log_clone.clone();
          ctx
            .spawn_child(
              NoopSupervisor,
              MailboxOptions::default(),
              move |_, child_msg: Message| {
                if let Message::User(child_value) = child_msg {
                  child_log.borrow_mut().push((child_value, 0));
                }
              },
            )
            .try_send_with_priority(Message::User(7), 0)
            .unwrap();
        }
      }
      Message::System(_) => {}
    }),
  )
  .unwrap();

  actor_ref.try_send_with_priority(Message::User(10), 1).unwrap();
  actor_ref.try_send_with_priority(Message::User(99), 7).unwrap();
  actor_ref.try_send_with_priority(Message::User(20), 3).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();
  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(scheduler.actor_count(), 2);

  assert_eq!(log.borrow().as_slice(), &[(99, 7), (20, 3), (10, 1), (7, 0)]);
}

#[cfg(feature = "std")]
#[test]
fn scheduler_prioritizes_system_messages() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler = PriorityScheduler::new(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| {
      log_clone.borrow_mut().push(msg.clone());
    }),
  )
  .unwrap();

  actor_ref
    .try_send_with_priority(Message::User(42), DEFAULT_PRIORITY)
    .unwrap();

  let control_envelope = PriorityEnvelope::from_system(SystemMessage::Stop).map(Message::System);
  actor_ref.try_send_envelope(control_envelope).unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(
    log.borrow().as_slice(),
    &[
      Message::System(SystemMessage::Stop),
      Message::System(SystemMessage::Watch(ActorId::ROOT)),
      Message::User(42),
    ]
  );
}

#[cfg(feature = "std")]
#[test]
fn priority_actor_ref_sends_system_messages() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<SystemMessage, _> = PriorityScheduler::new(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<SystemMessage>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(|sys| sys),
    handler_from_fn(move |_, msg: SystemMessage| {
      log_clone.borrow_mut().push(msg.clone());
    }),
  )
  .unwrap();

  actor_ref.try_send_system(SystemMessage::Restart).unwrap();
  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(
    log.borrow().as_slice(),
    &[SystemMessage::Restart, SystemMessage::Watch(ActorId::ROOT)]
  );
}

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
#[test]
fn scheduler_notifies_guardian_and_restarts_on_panic() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysRestart> =
    PriorityScheduler::new(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();
  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| {
      match msg {
        Message::System(SystemMessage::Watch(_)) => {
          // Watch メッセージは監視登録のみなのでログに残さない
        }
        Message::User(_) if panic_flag.get() => {
          panic_flag.set(false);
          panic!("boom");
        }
        _ => {
          log_clone.borrow_mut().push(msg.clone());
        }
      }
    }),
  )
  .unwrap();

  actor_ref
    .try_send_with_priority(Message::User(1), DEFAULT_PRIORITY)
    .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();
  assert!(log.borrow().is_empty());

  block_on(scheduler.dispatch_next()).unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::System(SystemMessage::Restart)]);
  assert!(!should_panic.get());
}

#[cfg(feature = "std")]
#[test]
fn scheduler_run_until_processes_messages() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysRestart> =
    PriorityScheduler::new(factory.clone(), Extensions::new());

  let log: Rc<RefCell<Vec<Message>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| match msg {
      Message::User(value) => log_clone.borrow_mut().push(Message::User(value)),
      Message::System(_) => {}
    }),
  )
  .unwrap();

  actor_ref
    .try_send_with_priority(Message::User(11), DEFAULT_PRIORITY)
    .unwrap();

  let mut loops = 0;
  futures::executor::block_on(scheduler.run_until(|| {
    let continue_loop = loops == 0;
    loops += 1;
    continue_loop
  }))
  .unwrap();

  assert_eq!(log.borrow().as_slice(), &[Message::User(11)]);
}

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
#[test]
fn scheduler_records_escalations() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysEscalate> =
    PriorityScheduler::with_strategy(factory.clone(), AlwaysEscalate, Extensions::new());

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
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| match msg {
      Message::System(SystemMessage::Watch(_)) => {}
      Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("boom");
      }
      _ => {}
    }),
  )
  .unwrap();

  actor_ref
    .try_send_with_priority(Message::User(1), DEFAULT_PRIORITY)
    .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  let handler_data = sink.borrow();
  assert_eq!(handler_data.len(), 1);
  assert_eq!(handler_data[0].actor, ActorId(0));
  let description = handler_data[0].description();
  assert!(description.starts_with("panic:"));

  // handler で除去済みのため take_escalations は空
  assert!(scheduler.take_escalations().is_empty());
}

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
#[test]
fn scheduler_escalation_handler_delivers_to_parent() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysEscalate> =
    PriorityScheduler::with_strategy(factory.clone(), AlwaysEscalate, Extensions::new());

  let (parent_mailbox, parent_sender) = factory.build_default_mailbox::<PriorityEnvelope<Message>>();
  let parent_ref: InternalActorRef<Message, TestMailboxRuntime> = InternalActorRef::new(parent_sender);
  scheduler.set_parent_guardian(parent_ref, MapSystemShared::new(Message::System));

  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| match msg {
      Message::System(SystemMessage::Watch(_)) => {}
      Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("boom");
      }
      _ => {}
    }),
  )
  .unwrap();

  actor_ref
    .try_send_with_priority(Message::User(1), DEFAULT_PRIORITY)
    .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  let envelope = parent_mailbox.queue().poll().unwrap().unwrap();
  let (msg, _, channel) = envelope.into_parts_with_channel();
  assert_eq!(channel, crate::runtime::mailbox::PriorityChannel::Control);
  match msg {
    Message::System(SystemMessage::Escalate(info)) => {
      assert_eq!(info.actor, ActorId(0));
      assert!(info.description().contains("panic"));
    }
    other => panic!("unexpected message: {:?}", other),
  }
}

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
#[test]
fn scheduler_escalation_chain_reaches_root() {
  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysEscalate> =
    PriorityScheduler::with_strategy(factory.clone(), AlwaysEscalate, Extensions::new());

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
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |ctx, msg: Message| match msg {
      Message::System(_) => {}
      Message::User(0) if !trigger_flag.get() => {
        trigger_flag.set(true);
        let panic_once = child_flag.clone();
        ctx
          .spawn_child(
            NoopSupervisor,
            MailboxOptions::default(),
            move |_, child_msg: Message| match child_msg {
              Message::System(_) => {}
              Message::User(1) if panic_once.get() => {
                panic_once.set(false);
                panic!("child failure");
              }
              _ => {}
            },
          )
          .try_send_with_priority(Message::User(1), DEFAULT_PRIORITY)
          .unwrap();
      }
      _ => {}
    }),
  )
  .unwrap();

  block_on(scheduler.dispatch_next()).unwrap();

  {
    let snapshot = collected.borrow();
    assert_eq!(snapshot.len(), 0);
  }

  parent_ref
    .try_send_with_priority(Message::User(0), DEFAULT_PRIORITY)
    .unwrap();

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
  let first_stage = first_failure.stage;
  assert!(first_stage.hops() >= 1, "child escalation should have hop count >= 1");

  let parent_failure = first_failure
    .escalate_to_parent()
    .expect("parent failure info must exist");
  let parent_stage = parent_failure.stage;
  assert!(
    parent_stage.hops() >= first_stage.hops(),
    "parent escalation hop count must be monotonic"
  );

  let mut current = parent_failure.clone();
  let mut root_failure = current.clone();
  while let Some(next) = current.escalate_to_parent() {
    root_failure = next.clone();
    current = next;
  }
  let root_stage = root_failure.stage;

  assert_eq!(first_failure.path.segments().last().copied(), Some(first_failure.actor));

  assert_eq!(
    parent_failure.actor,
    first_failure
      .path
      .segments()
      .first()
      .copied()
      .unwrap_or(first_failure.actor)
  );

  assert_eq!(root_failure.actor, parent_failure.actor);
  assert!(root_failure.path.is_empty());
  assert_eq!(root_failure.description(), parent_failure.description());
  assert!(
    root_stage.hops() >= parent_stage.hops(),
    "root escalation hop count must be monotonic"
  );
}

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
#[test]
fn scheduler_root_escalation_handler_invoked() {
  use std::sync::{Arc as StdArc, Mutex};

  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysEscalate> =
    PriorityScheduler::with_strategy(factory.clone(), AlwaysEscalate, Extensions::new());

  let events: StdArc<Mutex<Vec<FailureInfo>>> = StdArc::new(Mutex::new(Vec::new()));
  let events_clone = events.clone();

  scheduler.set_root_escalation_handler(Some(FailureEventHandler::new(move |info: &FailureInfo| {
    events_clone.lock().unwrap().push(info.clone());
  })));

  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| match msg {
      Message::System(SystemMessage::Watch(_)) => {}
      Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("root boom");
      }
      _ => {}
    }),
  )
  .unwrap();

  actor_ref
    .try_send_with_priority(Message::User(42), DEFAULT_PRIORITY)
    .unwrap();

  let events_ref = events.clone();
  block_on(scheduler.run_until(|| events_ref.lock().unwrap().is_empty())).unwrap();

  let events = events.lock().unwrap();
  assert_eq!(events.len(), 1);
  assert!(!events[0].description().is_empty());
}

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
#[test]
fn scheduler_requeues_failed_custom_escalation() {
  use core::cell::Cell;

  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysEscalate> =
    PriorityScheduler::with_strategy(factory.clone(), AlwaysEscalate, Extensions::new());

  let attempts = Rc::new(Cell::new(0usize));
  let attempts_clone = attempts.clone();
  scheduler.on_escalation(move |info| {
    assert!(
      info.stage.hops() >= 1,
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
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| match msg {
      Message::System(_) => {}
      Message::User(_) if panic_once.get() => {
        panic_once.set(false);
        panic!("custom escalation failure");
      }
      _ => {}
    }),
  )
  .unwrap();

  // consume initial watch message
  block_on(scheduler.dispatch_next()).unwrap();

  actor_ref
    .try_send_with_priority(Message::User(7), DEFAULT_PRIORITY)
    .unwrap();

  // first dispatch: panic occurs and escalation handler fails, causing requeue.
  block_on(scheduler.dispatch_next()).unwrap();
  assert_eq!(attempts.get(), 1);

  // second dispatch: retry succeeds and escalation queue drains.
  block_on(scheduler.dispatch_next()).unwrap();
  assert_eq!(attempts.get(), 2);
  assert!(scheduler.take_escalations().is_empty());
}

#[cfg(all(feature = "std", feature = "unwind-supervision"))]
#[test]
fn scheduler_root_event_listener_broadcasts() {
  use crate::api::tests::TestFailureEventStream;
  use crate::FailureEventStream;
  use std::sync::{Arc as StdArc, Mutex};

  let factory = TestMailboxRuntime::unbounded();
  let mut scheduler: PriorityScheduler<Message, _, AlwaysEscalate> =
    PriorityScheduler::with_strategy(factory.clone(), AlwaysEscalate, Extensions::new());

  let hub = TestFailureEventStream::default();
  let received: StdArc<Mutex<Vec<FailureInfo>>> = StdArc::new(Mutex::new(Vec::new()));
  let received_clone = received.clone();

  let _subscription = hub.subscribe(FailureEventListener::new(move |event| match event {
    crate::FailureEvent::RootEscalated(info) => {
      received_clone.lock().unwrap().push(info.clone());
    }
  }));

  scheduler.set_root_event_listener(Some(hub.listener()));

  let should_panic = Rc::new(Cell::new(true));
  let panic_flag = should_panic.clone();

  let actor_ref = spawn_with_runtime(
    &mut scheduler,
    factory.clone(),
    Box::new(NoopSupervisor),
    MailboxOptions::default(),
    MapSystemShared::new(Message::System),
    handler_from_fn(move |_, msg: Message| match msg {
      Message::System(SystemMessage::Watch(_)) => {}
      Message::User(_) if panic_flag.get() => {
        panic_flag.set(false);
        panic!("hub boom");
      }
      _ => {}
    }),
  )
  .unwrap();

  actor_ref
    .try_send_with_priority(Message::User(7), DEFAULT_PRIORITY)
    .unwrap();

  let received_ref = received.clone();
  block_on(scheduler.run_until(|| received_ref.lock().unwrap().is_empty())).unwrap();

  let events = received.lock().unwrap();
  assert_eq!(events.len(), 1);
  assert!(!events[0].description().is_empty());
}

#[cfg(feature = "std")]
#[test]
fn drive_ready_queue_worker_processes_actions() {
  use core::future::Future;
  use core::pin::Pin;
  use core::task::{Context, Poll};
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

  struct DummyWorker {
    state: Arc<Mutex<(VecDeque<WorkerAction>, Option<LocalBoxFuture<'static, usize>>, bool)>>,
    processed: Arc<Mutex<Vec<u32>>>,
  }

  #[derive(Clone)]
  enum WorkerAction {
    Progress(u32),
    Wait,
    End,
  }

  impl DummyWorker {
    fn new(actions: VecDeque<WorkerAction>, processed: Arc<Mutex<Vec<u32>>>) -> Self {
      Self {
        state: Arc::new(Mutex::new((actions, None, false))),
        processed,
      }
    }
  }

  impl ReadyQueueWorker<DynMessage, TestMailboxRuntime> for DummyWorker {
    fn process_ready_once(&self) -> Result<Option<bool>, QueueError<PriorityEnvelope<DynMessage>>> {
      let mut state = self.state.lock().unwrap();
      let (actions, wait_future, finished) = &mut *state;
      if let Some(action) = actions.pop_front() {
        match action {
          WorkerAction::Progress(value) => {
            self.processed.lock().unwrap().push(value);
            Ok(Some(true))
          }
          WorkerAction::Wait => {
            *wait_future = Some(futures::future::ready(0usize).boxed_local());
            Ok(None)
          }
          WorkerAction::End => {
            *finished = true;
            Ok(None)
          }
        }
      } else {
        Ok(None)
      }
    }

    fn wait_for_ready(&self) -> Option<LocalBoxFuture<'static, usize>> {
      let mut state = self.state.lock().unwrap();
      let (_, wait_future, finished) = &mut *state;
      if let Some(fut) = wait_future.take() {
        Some(fut)
      } else if *finished {
        None
      } else {
        None
      }
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
  let actions = VecDeque::from(vec![
    WorkerAction::Progress(1),
    WorkerAction::Wait,
    WorkerAction::Progress(2),
    WorkerAction::End,
  ]);
  let worker_impl = DummyWorker::new(actions, processed.clone());
  let worker =
    ArcShared::new(worker_impl).into_dyn(|inner| inner as &dyn ReadyQueueWorker<DynMessage, TestMailboxRuntime>);

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
          // LocalPool 上で他タスクに制御を明示的に渡すため即時完了 Future ではなく 1 回だけ Pending になる Future を使う
          YieldOnce { yielded: false }
        },
        move || shutdown_poll_future(shutdown_for_wait.clone()),
      )
      .map(|res| res.expect("worker loop succeeds")),
    )
    .expect("spawn worker loop");

  let shutdown_trigger = shutdown.clone();
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
