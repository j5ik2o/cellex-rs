extern crate alloc;

use alloc::vec::Vec;

use cellex_utils_core_rs::{
  collections::queue::backend::{OverflowPolicy, QueueError},
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
};
use spin::RwLock;

use super::*;
use crate::{
  api::{
    actor::{ActorId, ActorPath},
    actor_runtime::GenericActorRuntime,
    mailbox::{
      messages::SystemMessage,
      queue_mailbox::{QueueMailbox, SyncMailbox, SyncMailboxQueue, SystemMailboxQueue},
    },
    process::{
      dead_letter::{DeadLetterListener, DeadLetterReason},
      pid::SystemId,
    },
    test_support::{TestMailboxFactory, TestSignal},
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MessageEnvelope},
  },
};

#[test]
fn actor_ref_routes_drop_newest_to_dead_letter() {
  type TestRuntime = GenericActorRuntime<TestMailboxFactory>;

  let queue = SystemMailboxQueue::new(SyncMailboxQueue::bounded(1, OverflowPolicy::DropNewest), None);
  let mailbox: SyncMailbox<PriorityEnvelope<AnyMessage>, TestSignal> = QueueMailbox::new(queue, TestSignal::default());
  let priority_ref: PriorityActorRef<AnyMessage, TestMailboxFactory> = PriorityActorRef::new(mailbox.producer());

  let registry: ArcShared<ActorProcessRegistry<TestRuntime>> =
    ArcShared::new(ProcessRegistry::new(SystemId::new("deadletter-test"), None));

  let observed = ArcShared::new(SpinSyncMutex::new(Vec::new()));
  let listener = {
    let observed = observed.clone();
    ArcShared::new(
      move |letter: &crate::api::process::dead_letter::DeadLetter<ArcShared<PriorityEnvelope<AnyMessage>>>| {
        observed.lock().push(letter.reason.clone());
      },
    )
    .into_dyn(|f| f as &DeadLetterListener<ArcShared<PriorityEnvelope<AnyMessage>>>)
  };
  registry.with_ref(|reg| reg.subscribe_dead_letters(listener));

  let path = ActorPath::new().push_child(ActorId(1));
  let handle = ArcShared::new(priority_ref.clone());
  let pid = registry.with_ref(|reg| reg.register_local(path, handle));

  let pid_slot = ArcShared::new(RwLock::new(None));
  let actor_ref: ActorRef<u32, TestRuntime> =
    ActorRef::new(priority_ref.clone(), pid_slot.clone(), Some(registry.clone()));
  actor_ref.set_pid(pid.clone());

  let first_message = PriorityEnvelope::with_default_priority(AnyMessage::new(MessageEnvelope::user(0u32)));
  mailbox.try_send_mailbox(first_message).expect("warm-up enqueue succeeds");

  let result = actor_ref.tell(1u32);
  assert!(matches!(result, Err(QueueError::Full(_))));

  let recorded = observed.lock().clone();
  assert!(
    recorded.iter().any(|reason| matches!(reason, DeadLetterReason::DeliveryRejected)),
    "expected delivery rejection dead letter, got {recorded:?}"
  );
}

#[test]
fn actor_ref_tell_with_priority_rejects_and_routes_dead_letter() {
  type TestRuntime = GenericActorRuntime<TestMailboxFactory>;

  let queue = SystemMailboxQueue::new(SyncMailboxQueue::bounded(1, OverflowPolicy::DropNewest), None);
  let mailbox: SyncMailbox<PriorityEnvelope<AnyMessage>, TestSignal> = QueueMailbox::new(queue, TestSignal::default());
  let priority_ref: PriorityActorRef<AnyMessage, TestMailboxFactory> = PriorityActorRef::new(mailbox.producer());

  let registry: ArcShared<ActorProcessRegistry<TestRuntime>> =
    ArcShared::new(ProcessRegistry::new(SystemId::new("deadletter-test"), None));

  let observed = ArcShared::new(SpinSyncMutex::new(Vec::new()));
  let listener = {
    let observed = observed.clone();
    ArcShared::new(
      move |letter: &crate::api::process::dead_letter::DeadLetter<ArcShared<PriorityEnvelope<AnyMessage>>>| {
        observed.lock().push(letter.reason.clone());
      },
    )
    .into_dyn(|f| f as &DeadLetterListener<ArcShared<PriorityEnvelope<AnyMessage>>>)
  };
  registry.with_ref(|reg| reg.subscribe_dead_letters(listener));

  let path = ActorPath::new().push_child(ActorId(2));
  let handle = ArcShared::new(priority_ref.clone());
  let pid = registry.with_ref(|reg| reg.register_local(path, handle));

  let pid_slot = ArcShared::new(RwLock::new(None));
  let actor_ref: ActorRef<u32, TestRuntime> =
    ActorRef::new(priority_ref.clone(), pid_slot.clone(), Some(registry.clone()));
  actor_ref.set_pid(pid.clone());

  let first_message = PriorityEnvelope::with_default_priority(AnyMessage::new(MessageEnvelope::user(0u32)));
  mailbox.try_send_mailbox(first_message).expect("warm-up enqueue succeeds");

  let result = actor_ref.tell_with_priority(1u32, 5);
  assert!(matches!(result, Err(QueueError::Full(_))));

  let recorded = observed.lock().clone();
  assert!(
    recorded.iter().any(|reason| matches!(reason, DeadLetterReason::DeliveryRejected)),
    "expected delivery rejection dead letter, got {recorded:?}"
  );
}

#[test]
fn actor_ref_send_system_routes_dead_letter_on_overflow() {
  type TestRuntime = GenericActorRuntime<TestMailboxFactory>;

  let queue = SystemMailboxQueue::new(SyncMailboxQueue::bounded(1, OverflowPolicy::DropNewest), None);
  let mailbox: SyncMailbox<PriorityEnvelope<AnyMessage>, TestSignal> = QueueMailbox::new(queue, TestSignal::default());
  let priority_ref: PriorityActorRef<AnyMessage, TestMailboxFactory> = PriorityActorRef::new(mailbox.producer());

  let registry: ArcShared<ActorProcessRegistry<TestRuntime>> =
    ArcShared::new(ProcessRegistry::new(SystemId::new("deadletter-test"), None));

  let observed = ArcShared::new(SpinSyncMutex::new(Vec::new()));
  let listener = {
    let observed = observed.clone();
    ArcShared::new(
      move |letter: &crate::api::process::dead_letter::DeadLetter<ArcShared<PriorityEnvelope<AnyMessage>>>| {
        observed.lock().push(letter.reason.clone());
      },
    )
    .into_dyn(|f| f as &DeadLetterListener<ArcShared<PriorityEnvelope<AnyMessage>>>)
  };
  registry.with_ref(|reg| reg.subscribe_dead_letters(listener));

  let path = ActorPath::new().push_child(ActorId(3));
  let handle = ArcShared::new(priority_ref.clone());
  let pid = registry.with_ref(|reg| reg.register_local(path, handle));

  let pid_slot = ArcShared::new(RwLock::new(None));
  let actor_ref: ActorRef<u32, TestRuntime> =
    ActorRef::new(priority_ref.clone(), pid_slot.clone(), Some(registry.clone()));
  actor_ref.set_pid(pid.clone());

  let first_message = PriorityEnvelope::with_default_priority(AnyMessage::new(MessageEnvelope::user(0u32)));
  mailbox.try_send_mailbox(first_message).expect("warm-up enqueue succeeds");

  let result = actor_ref.send_system(SystemMessage::Stop);
  assert!(matches!(result, Err(QueueError::Full(_))));

  let recorded = observed.lock().clone();
  assert!(
    recorded.iter().any(|reason| matches!(reason, DeadLetterReason::DeliveryRejected)),
    "expected delivery rejection dead letter, got {recorded:?}"
  );
}
