#![cfg(feature = "std")]
#![allow(deprecated, unused_imports)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::disallowed_types)]

use alloc::{rc::Rc, vec::Vec};
use core::cell::RefCell;

use cellex_utils_core_rs::{sync::ArcShared, QueueError, Shared, DEFAULT_PRIORITY};
#[cfg(feature = "std")]
use futures::executor::block_on;
use spin::RwLock;

use super::*;
use crate::{
  api::{
    actor::ActorPath,
    actor_runtime::{GenericActorRuntime, MailboxConcurrencyOf},
    actor_system::map_system::MapSystemShared,
    guardian::AlwaysRestart,
    mailbox::{MailboxOptions, PriorityEnvelope, SystemMessage},
    messaging::{AnyMessage, MessageEnvelope, MessageMetadata},
    process::{
      dead_letter::{DeadLetter, DeadLetterListener, DeadLetterReason},
      pid::{NodeId, Pid, SystemId},
      process_registry::ProcessResolution,
    },
    test_support::TestMailboxFactory,
  },
  internal::actor::InternalProps,
};

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
enum Message {
  User(u32),
  System,
}

#[cfg(feature = "std")]
#[test]
fn actor_system_spawns_and_processes_messages() {
  let mailbox_factory = TestMailboxFactory::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_factory);
  let mut system: InternalActorSystem<_, AlwaysRestart> = InternalActorSystem::new(actor_runtime);

  let map_system = MapSystemShared::new(|_: SystemMessage| AnyMessage::new(Message::System));
  let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
  let log_clone = log.clone();

  let _process_registry = system.process_registry();
  let mut root = system.root_context();
  let actor_ref = root
    .spawn(InternalProps::new(MailboxOptions::default(), map_system.clone(), move |_, msg: AnyMessage| {
      let Ok(message) = msg.downcast::<Message>() else {
        panic!("unexpected message type");
      };
      match message {
        | Message::User(value) => log_clone.borrow_mut().push(value),
        | Message::System => {},
      }
      Ok(())
    }))
    .expect("spawn actor");

  actor_ref.try_send_with_priority(AnyMessage::new(Message::User(7)), DEFAULT_PRIORITY).expect("send message");

  block_on(root.dispatch_next()).expect("dispatch");

  assert_eq!(log.borrow().as_slice(), &[7]);
}

#[cfg(feature = "std")]
#[test]
fn process_registry_registers_and_deregisters_actor() {
  #[derive(Debug, Clone)]
  enum TestMsg {
    CapturePid,
  }

  let mailbox_factory = TestMailboxFactory::unbounded();
  let actor_runtime = GenericActorRuntime::new(mailbox_factory);
  let mut system: InternalActorSystem<_, AlwaysRestart> = InternalActorSystem::new(actor_runtime);

  let map_system = MapSystemShared::new(|_: SystemMessage| AnyMessage::new(Message::System));
  let captured: Rc<RefCell<Option<Pid>>> = Rc::new(RefCell::new(None));
  let captured_clone = captured.clone();

  let registry = system.process_registry();
  let mut root = system.root_context();
  let actor_ref = root
    .spawn(InternalProps::new(MailboxOptions::default(), map_system.clone(), move |ctx, msg: AnyMessage| {
      if let Ok(TestMsg::CapturePid) = msg.downcast::<TestMsg>() {
        captured_clone.borrow_mut().replace(ctx.pid().clone());
      }
      Ok(())
    }))
    .expect("spawn actor");

  actor_ref.try_send_with_priority(AnyMessage::new(TestMsg::CapturePid), DEFAULT_PRIORITY).expect("send capture");

  block_on(root.dispatch_next()).expect("dispatch capture");

  drop(root);
  let pid = captured.borrow().clone().expect("pid captured");
  let resolution = registry.with_ref(|registry| registry.resolve_pid(&pid));
  assert!(matches!(resolution, ProcessResolution::Local(_)));

  let map_clone = map_system.clone();
  actor_ref
    .try_send_envelope(PriorityEnvelope::from_system(SystemMessage::Stop).map(|sys| map_clone(sys)))
    .expect("send stop");

  let mut root = system.root_context();
  block_on(root.dispatch_next()).ok();
  block_on(root.dispatch_next()).ok();
  drop(root);

  let resolution_after = registry.with_ref(|registry| registry.resolve_pid(&pid));
  assert!(matches!(resolution_after, ProcessResolution::Unresolved));
}

#[cfg(feature = "std")]
#[test]
fn responder_pid_allows_response_delivery() {
  type TestRuntime = GenericActorRuntime<TestMailboxFactory>;
  #[derive(Debug, Clone, PartialEq, Eq)]
  enum TestMsg {
    RecordPid,
    Ping,
    Response(u32),
  }

  let mailbox_factory = TestMailboxFactory::unbounded();
  let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_factory);
  let mut system: InternalActorSystem<_, AlwaysRestart> = InternalActorSystem::new(actor_runtime);

  let map_system = MapSystemShared::new(|_: SystemMessage| AnyMessage::new(Message::System));
  let probe_pid: Rc<RefCell<Option<Pid>>> = Rc::new(RefCell::new(None));
  let probe_pid_clone = probe_pid.clone();
  let responses: Rc<RefCell<Vec<TestMsg>>> = Rc::new(RefCell::new(Vec::new()));
  let responses_clone = responses.clone();

  let process_registry = system.process_registry();
  let mut root = system.root_context();
  let probe_ref = root
    .spawn(InternalProps::new(MailboxOptions::default(), map_system.clone(), move |ctx, msg: AnyMessage| {
      if let Ok(envelope) = msg.downcast::<MessageEnvelope<TestMsg>>() {
        match envelope {
          | MessageEnvelope::User(user) => {
            let (message, _) = user.into_parts::<MailboxConcurrencyOf<TestRuntime>>();
            match message {
              | TestMsg::RecordPid => {
                probe_pid_clone.borrow_mut().replace(ctx.pid().clone());
              },
              | TestMsg::Response(value) => {
                responses_clone.borrow_mut().push(TestMsg::Response(value));
              },
              | TestMsg::Ping => {},
            }
          },
          | MessageEnvelope::System(_) => {},
        }
      }
      Ok(())
    }))
    .expect("spawn probe");

  let target_ref = root
    .spawn(InternalProps::new(MailboxOptions::default(), map_system.clone(), move |ctx, msg: AnyMessage| {
      if let Ok(envelope) = msg.downcast::<MessageEnvelope<TestMsg>>() {
        match envelope {
          | MessageEnvelope::User(user) => {
            let (message, metadata) = user.into_parts::<MailboxConcurrencyOf<TestRuntime>>();
            if matches!(message, TestMsg::Ping) {
              if let Some(metadata) = metadata {
                if let Some(responder_pid) = metadata.responder_pid().cloned() {
                  ctx.process_registry().with_ref(|registry| {
                    if let ProcessResolution::Local(handle) = registry.resolve_pid(&responder_pid) {
                      let response = AnyMessage::new(MessageEnvelope::user(TestMsg::Response(99)));
                      let priority = PriorityEnvelope::with_default_priority(response);
                      let _ = handle.with_ref(|actor_ref| actor_ref.clone()).try_send_envelope(priority);
                    }
                  });
                }
              }
            }
          },
          | MessageEnvelope::System(_) => {},
        }
      }
      Ok(())
    }))
    .expect("spawn target");

  let probe_slot = ArcShared::new(RwLock::new(None));
  let probe_typed: crate::api::actor::actor_ref::ActorRef<TestMsg, TestRuntime> =
    crate::api::actor::actor_ref::ActorRef::new(probe_ref.clone(), probe_slot, Some(process_registry.clone()));
  probe_typed.tell(TestMsg::RecordPid).expect("record pid message");

  block_on(root.dispatch_next()).expect("dispatch record pid");

  let responder_pid = probe_pid.borrow().clone().expect("pid available");

  let target_slot = ArcShared::new(RwLock::new(None));
  let target_typed: crate::api::actor::actor_ref::ActorRef<TestMsg, TestRuntime> =
    crate::api::actor::actor_ref::ActorRef::new(target_ref.clone(), target_slot, Some(process_registry.clone()));
  let metadata = MessageMetadata::<MailboxConcurrencyOf<TestRuntime>>::new().with_responder_pid(responder_pid);
  target_typed.tell_with_metadata(TestMsg::Ping, metadata).expect("send ping");

  block_on(root.dispatch_next()).expect("dispatch ping");
  block_on(root.dispatch_next()).ok();

  assert_eq!(responses.borrow().as_slice(), &[TestMsg::Response(99)]);
}

#[test]
fn actor_ref_emits_dead_letter_on_unregistered_pid() {
  type TestRuntime = GenericActorRuntime<TestMailboxFactory>;

  let mailbox_factory = TestMailboxFactory::unbounded();
  let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_factory);
  let mut system: InternalActorSystem<_, AlwaysRestart> = InternalActorSystem::new(actor_runtime);

  let process_registry = system.process_registry();
  let observed_reasons = ArcShared::new(RwLock::new(Vec::<DeadLetterReason>::new()));
  let observed_clone = observed_reasons.clone();
  let listener = ArcShared::new(move |letter: &DeadLetter<ArcShared<PriorityEnvelope<AnyMessage>>>| {
    observed_clone.write().push(letter.reason.clone());
  })
  .into_dyn(|f| f as &DeadLetterListener<ArcShared<PriorityEnvelope<AnyMessage>>>);
  process_registry.with_ref(|registry| registry.subscribe_dead_letters(listener));

  let mut root = system.root_context();
  let map_system =
    MapSystemShared::new(|sys: SystemMessage| AnyMessage::new(MessageEnvelope::<u32>::System(sys.clone())));
  let captured_pid = ArcShared::new(RwLock::new(None));
  let captured_pid_clone = captured_pid.clone();
  let actor_ref_raw = root
    .spawn(InternalProps::new(MailboxOptions::default(), map_system, move |ctx, msg: AnyMessage| {
      if let Ok(envelope) = msg.downcast::<MessageEnvelope<u32>>() {
        if let MessageEnvelope::User(_) = envelope {
          captured_pid_clone.write().replace(ctx.pid().clone());
        }
      }
      Ok(())
    }))
    .expect("spawn actor");

  let pid_slot = ArcShared::new(RwLock::new(None));
  let typed_ref: crate::api::actor::actor_ref::ActorRef<u32, TestRuntime> =
    crate::api::actor::actor_ref::ActorRef::new(actor_ref_raw, pid_slot, Some(process_registry.clone()));

  typed_ref.tell(0).expect("capture pid message");
  block_on(root.dispatch_next()).expect("dispatch capture pid");

  let pid = captured_pid.read().clone().expect("pid assigned");
  typed_ref.set_pid(pid.clone());
  process_registry.with_ref(|registry| registry.deregister(&pid));
  let resolution_after = process_registry.with_ref(|registry| registry.resolve_pid(&pid));
  assert!(matches!(resolution_after, ProcessResolution::Unresolved));

  let send_result = typed_ref.tell(7);
  match send_result {
    | Ok(()) => panic!("expected disconnected error"),
    | Err(QueueError::Disconnected) => {},
    | Err(other) => panic!("unexpected error: {other:?}"),
  }

  let reasons = observed_reasons.read();
  assert_eq!(reasons.as_slice(), &[DeadLetterReason::UnregisteredPid]);
}

#[test]
fn actor_ref_records_network_unreachable_for_remote_pid() {
  type TestRuntime = GenericActorRuntime<TestMailboxFactory>;

  let mailbox_factory = TestMailboxFactory::unbounded();
  let actor_runtime: TestRuntime = GenericActorRuntime::new(mailbox_factory);
  let mut system: InternalActorSystem<_, AlwaysRestart> = InternalActorSystem::new(actor_runtime);

  let process_registry = system.process_registry();
  let observed = ArcShared::new(RwLock::new(Vec::<DeadLetterReason>::new()));
  let observed_clone = observed.clone();
  let listener = ArcShared::new(move |letter: &DeadLetter<ArcShared<PriorityEnvelope<AnyMessage>>>| {
    observed_clone.write().push(letter.reason.clone());
  })
  .into_dyn(|f| f as &DeadLetterListener<ArcShared<PriorityEnvelope<AnyMessage>>>);
  process_registry.with_ref(|registry| registry.subscribe_dead_letters(listener));

  let mut root = system.root_context();
  let map_system =
    MapSystemShared::new(|sys: SystemMessage| AnyMessage::new(MessageEnvelope::<u32>::System(sys.clone())));
  let actor_ref_raw = root
    .spawn(InternalProps::new(MailboxOptions::default(), map_system, |_ctx, _msg: AnyMessage| Ok(())))
    .expect("spawn actor");
  let pid_slot = ArcShared::new(RwLock::new(None));
  let typed_ref: crate::api::actor::actor_ref::ActorRef<u32, TestRuntime> =
    crate::api::actor::actor_ref::ActorRef::new(actor_ref_raw, pid_slot, Some(process_registry.clone()));

  let remote_pid = Pid::new(SystemId::new("remote"), ActorPath::new()).with_node(NodeId::new("node2", Some(2552)));
  typed_ref.set_pid(remote_pid);

  let send_result = typed_ref.tell(42);
  assert!(matches!(send_result, Err(QueueError::Disconnected)));

  let reasons = observed.read();
  assert!(reasons.contains(&DeadLetterReason::NetworkUnreachable));
}
