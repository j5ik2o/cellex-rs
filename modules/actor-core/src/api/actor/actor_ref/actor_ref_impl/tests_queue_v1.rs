#![cfg(feature = "queue-v1")]

extern crate alloc;

use alloc::vec::Vec;

use cellex_utils_core_rs::{
  collections::queue::QueueError,
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
};
use spin::RwLock;

use super::*;
use crate::{
  api::{
    actor::{ActorId, ActorPath},
    actor_runtime::GenericActorRuntime,
    metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
    process::{
      dead_letter::{DeadLetterListener, DeadLetterReason},
      pid::SystemId,
    },
    test_support::TestMailboxFactory,
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MessageEnvelope},
  },
};

struct RecordingSink {
  events: ArcShared<SpinSyncMutex<Vec<MetricsEvent>>>,
}

impl RecordingSink {
  fn new() -> (Self, ArcShared<SpinSyncMutex<Vec<MetricsEvent>>>) {
    let events = ArcShared::new(SpinSyncMutex::new(Vec::new()));
    (Self { events: events.clone() }, events)
  }
}

impl MetricsSink for RecordingSink {
  fn record(&self, event: MetricsEvent) {
    self.events.lock().push(event);
  }
}

#[test]
fn actor_ref_dead_letter_and_metrics_on_queue_v1_overflow() {
  type TestRuntime = GenericActorRuntime<TestMailboxFactory>;

  let (mut mailbox, producer) =
    TestMailboxFactory::with_capacity_per_queue(1).build_mailbox::<PriorityEnvelope<AnyMessage>>(Default::default());
  let priority_ref: PriorityActorRef<AnyMessage, TestMailboxFactory> = PriorityActorRef::new(producer);

  let registry: ArcShared<ActorProcessRegistry<TestRuntime>> =
    ArcShared::new(ProcessRegistry::new(SystemId::new("queue-v1-deadletter"), None));

  let observed_dead_letters = ArcShared::new(SpinSyncMutex::new(Vec::new()));
  let listener = {
    let observed = observed_dead_letters.clone();
    ArcShared::new(
      move |letter: &crate::api::process::dead_letter::DeadLetter<ArcShared<PriorityEnvelope<AnyMessage>>>| {
        observed.lock().push(letter.reason.clone());
      },
    )
    .into_dyn(|f| f as &DeadLetterListener<ArcShared<PriorityEnvelope<AnyMessage>>>)
  };
  registry.with_ref(|reg| reg.subscribe_dead_letters(listener));

  let (metrics_sink, recorded_events) = RecordingSink::new();
  mailbox.set_metrics_sink(Some(MetricsSinkShared::new(metrics_sink)));

  let path = ActorPath::new().push_child(ActorId(10));
  let handle = ArcShared::new(priority_ref.clone());
  let pid = registry.with_ref(|reg| reg.register_local(path, handle));

  let pid_slot = ArcShared::new(RwLock::new(None));
  let actor_ref: ActorRef<u32, TestRuntime> =
    ActorRef::new(priority_ref.clone(), pid_slot.clone(), Some(registry.clone()));
  actor_ref.set_pid(pid.clone());

  let first_message = PriorityEnvelope::with_default_priority(AnyMessage::new(MessageEnvelope::user(0u32)));
  mailbox.try_send_mailbox(first_message).expect("first enqueue succeeds");

  let result = actor_ref.tell(1u32);
  assert!(matches!(result, Err(QueueError::Full(_))));

  let recorded = observed_dead_letters.lock().clone();
  assert!(
    recorded.iter().any(|reason| matches!(reason, DeadLetterReason::DeliveryRejected)),
    "expected DeliveryRejected dead letter, got {recorded:?}"
  );

  let metrics = recorded_events.lock().clone();
  assert!(
    metrics.iter().any(|event| matches!(event, MetricsEvent::MailboxEnqueued)),
    "expected MailboxEnqueued metric, got {metrics:?}"
  );
}
