#![allow(clippy::unwrap_used)]

extern crate std;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use core::time::Duration;
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
      ready_queue_coordinator::{InvokeResult, ResumeCondition, SuspendReason},
      ActorScheduler, ActorSchedulerSpawnContext,
    },
    extensions::Extensions,
    guardian::AlwaysRestart,
    mailbox::messages::SystemMessage,
    metrics::{SuspensionClock, SuspensionClockShared},
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

  let suspend_envelope = PriorityEnvelope::from_system(SystemMessage::Suspend).map(dyn_system);
  actor_ref.try_send_envelope(suspend_envelope).expect("send suspend");
  actor_ref.try_send_with_priority(dyn_user(9), DEFAULT_PRIORITY).unwrap();

  assert!(scheduler.drain_ready().unwrap());
  {
    let context = scheduler.context_for_testing();
    let mut ctx = context.lock();
    ctx.core.inject_invoke_result_for_testing(0, InvokeResult::Suspended {
      reason:    SuspendReason::Backpressure,
      resume_on: ResumeCondition::WhenCapacityAvailable,
    });
  }

  for _ in 0..5 {
    let _ = scheduler.drain_ready().unwrap();
    if !log.borrow().is_empty() {
      break;
    }
  }
  assert_eq!(log.borrow().as_slice(), &[9]);
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
