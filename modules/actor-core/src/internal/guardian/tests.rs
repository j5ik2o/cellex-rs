#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::disallowed_types)]

use super::*;
use crate::api::mailbox::{PriorityEnvelope, SystemMessage};
use crate::internal::actor::InternalActorRef;
use crate::internal::mailbox::test_support::TestMailboxRuntime;
use crate::ActorId;
use crate::ActorPath;
use crate::MailboxRuntime;
use crate::MapSystemShared;
use crate::PriorityChannel;
use crate::SupervisorDirective;
use crate::{ActorFailure, BehaviorFailure};
use crate::{ChildNaming, SpawnError};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use cellex_utils_core_rs::{Element, DEFAULT_PRIORITY};
use spin::Mutex;

#[test]
fn guardian_sends_restart_message() {
  let (mailbox, sender) = TestMailboxRuntime::unbounded().build_default_mailbox::<PriorityEnvelope<SystemMessage>>();
  let ref_control: InternalActorRef<SystemMessage, TestMailboxRuntime> = InternalActorRef::new(sender);

  let mut guardian: Guardian<SystemMessage, _, AlwaysRestart> = Guardian::new(AlwaysRestart);
  let parent_id = ActorId(1);
  let parent_path = ActorPath::new();
  let (actor_id, _path) = guardian
    .register_child(
      ref_control.clone(),
      MapSystemShared::new(|sys| sys),
      Some(parent_id),
      &parent_path,
    )
    .unwrap();

  let first_envelope = mailbox.queue().poll().unwrap().unwrap();
  assert_eq!(first_envelope.into_parts().0, SystemMessage::Watch(parent_id));

  assert!(guardian
    .notify_failure(actor_id, ActorFailure::from_message("panic"))
    .unwrap()
    .is_none());

  let envelope = mailbox.queue().poll().unwrap().unwrap();
  let (message, priority, channel) = envelope.into_parts_with_channel();
  assert_eq!(message, SystemMessage::Restart);
  assert!(priority > DEFAULT_PRIORITY);
  assert_eq!(channel, PriorityChannel::Control);
}

#[test]
fn guardian_sends_stop_message() {
  struct AlwaysStop;
  impl<M, R> GuardianStrategy<M, R> for AlwaysStop
  where
    M: Element,
    R: MailboxRuntime,
  {
    fn decide(&mut self, _actor: ActorId, _error: &dyn BehaviorFailure) -> SupervisorDirective {
      SupervisorDirective::Stop
    }
  }

  let (mailbox, sender) = TestMailboxRuntime::unbounded().build_default_mailbox::<PriorityEnvelope<SystemMessage>>();
  let ref_control: InternalActorRef<SystemMessage, TestMailboxRuntime> = InternalActorRef::new(sender);

  let mut guardian: Guardian<SystemMessage, _, AlwaysStop> = Guardian::new(AlwaysStop);
  let parent_id = ActorId(7);
  let parent_path = ActorPath::new();
  let (actor_id, _path) = guardian
    .register_child(
      ref_control.clone(),
      MapSystemShared::new(|sys| sys),
      Some(parent_id),
      &parent_path,
    )
    .unwrap();

  let watch_envelope = mailbox.queue().poll().unwrap().unwrap();
  assert_eq!(watch_envelope.into_parts().0, SystemMessage::Watch(parent_id));

  assert!(guardian
    .notify_failure(actor_id, ActorFailure::from_message("panic"))
    .unwrap()
    .is_none());

  let envelope = mailbox.queue().poll().unwrap().unwrap();
  assert_eq!(envelope.into_parts().0, SystemMessage::Stop);
}

#[test]
fn guardian_emits_unwatch_on_remove() {
  let (mailbox, sender) = TestMailboxRuntime::unbounded().build_default_mailbox::<PriorityEnvelope<SystemMessage>>();
  let ref_control: InternalActorRef<SystemMessage, TestMailboxRuntime> = InternalActorRef::new(sender);

  let mut guardian: Guardian<SystemMessage, _, AlwaysRestart> = Guardian::new(AlwaysRestart);
  let parent_id = ActorId(3);
  let parent_path = ActorPath::new();
  let (actor_id, _path) = guardian
    .register_child(
      ref_control.clone(),
      MapSystemShared::new(|sys| sys),
      Some(parent_id),
      &parent_path,
    )
    .unwrap();

  // consume watch message
  let _ = mailbox.queue().poll().unwrap().unwrap();

  let _ = guardian.remove_child(actor_id);

  let envelope = mailbox.queue().poll().unwrap().unwrap();
  assert_eq!(envelope.into_parts().0, SystemMessage::Unwatch(parent_id));
}

#[test]
fn guardian_strategy_receives_behavior_failure() {
  struct CaptureStrategy(Arc<Mutex<Vec<String>>>);

  impl<M, R> GuardianStrategy<M, R> for CaptureStrategy
  where
    M: Element,
    R: MailboxRuntime,
  {
    fn decide(&mut self, _actor: ActorId, error: &dyn BehaviorFailure) -> SupervisorDirective {
      self.0.lock().push(error.description().into_owned());
      SupervisorDirective::Resume
    }
  }

  let (_, sender) = TestMailboxRuntime::unbounded().build_default_mailbox::<PriorityEnvelope<SystemMessage>>();
  let ref_control: InternalActorRef<SystemMessage, TestMailboxRuntime> = InternalActorRef::new(sender);

  let captured = Arc::new(Mutex::new(Vec::new()));
  let mut guardian: Guardian<SystemMessage, _, CaptureStrategy> = Guardian::new(CaptureStrategy(captured.clone()));
  let parent_path = ActorPath::new();
  let (actor_id, _) = guardian
    .register_child(ref_control.clone(), MapSystemShared::new(|sys| sys), None, &parent_path)
    .unwrap();

  guardian
    .notify_failure(actor_id, ActorFailure::from_message("child boom"))
    .expect("notify succeeds");

  let log = captured.lock();
  assert_eq!(log.len(), 1);
  assert!(log[0].contains("child boom"));
}

#[test]
fn guardian_rejects_duplicate_names() {
  let (_mailbox, sender) = TestMailboxRuntime::unbounded().build_default_mailbox::<PriorityEnvelope<SystemMessage>>();
  let ref_control: InternalActorRef<SystemMessage, TestMailboxRuntime> = InternalActorRef::new(sender.clone());

  let mut guardian: Guardian<SystemMessage, _, AlwaysRestart> = Guardian::new(AlwaysRestart);
  let parent_path = ActorPath::new();
  guardian
    .register_child_with_naming(
      ref_control.clone(),
      MapSystemShared::new(|sys| sys),
      None,
      &parent_path,
      ChildNaming::Explicit("worker".to_string()),
    )
    .expect("first spawn");

  let err = guardian
    .register_child_with_naming(
      InternalActorRef::new(sender),
      MapSystemShared::new(|sys| sys),
      None,
      &parent_path,
      ChildNaming::Explicit("worker".to_string()),
    )
    .expect_err("second spawn must fail");

  match err {
    SpawnError::NameExists(name) => assert_eq!(name, "worker"),
    SpawnError::Queue(_) => panic!("unexpected queue error"),
  }
}
