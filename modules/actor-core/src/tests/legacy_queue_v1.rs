#![cfg(feature = "queue-v1")]

extern crate alloc;

use alloc::vec::Vec;

use cellex_utils_core_rs::{
  collections::queue::QueueError,
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
  Element, MpscQueue, Shared,
};
use spin::RwLock;

use crate::{
  api::{
    actor::{
      actor_ref::{ActorRef, PriorityActorRef},
      ActorId, ActorPath,
    },
    actor_runtime::{GenericActorRuntime, MailboxOf},
    mailbox::{
      messages::SystemMessage, queue_mailbox::QueueMailbox, MailboxFactory, MailboxOptions, MailboxPair,
      QueueMailboxProducer, SingleThread,
    },
    metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
    process::{
      dead_letter::{DeadLetterListener, DeadLetterReason},
      pid::SystemId,
      process_registry::ProcessRegistry,
    },
    test_support::{SharedBackendHandle, TestMailboxFactory, TestSignal},
  },
  shared::{
    mailbox::messages::PriorityEnvelope,
    messaging::{AnyMessage, MessageEnvelope},
  },
};

type ActorProcessRegistry<AR> =
  ProcessRegistry<PriorityActorRef<AnyMessage, MailboxOf<AR>>, ArcShared<PriorityEnvelope<AnyMessage>>>;

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

#[derive(Clone, Debug, Default)]
struct EmbeddedRcMailboxFactory {
  capacity: Option<usize>,
}

impl EmbeddedRcMailboxFactory {
  const fn new(capacity: Option<usize>) -> Self {
    Self { capacity }
  }

  const fn with_capacity(capacity: usize) -> Self {
    Self::new(Some(capacity))
  }

  fn resolve_capacity(&self, options: MailboxOptions) -> Option<usize> {
    match (options.capacity_limit(), self.capacity) {
      | (Some(limit), _) => Some(limit),
      | (None, injected) => injected,
    }
  }
}

impl MailboxFactory for EmbeddedRcMailboxFactory {
  type Concurrency = SingleThread;
  type Mailbox<M>
    = QueueMailbox<Self::Queue<M>, Self::Signal>
  where
    M: Element;
  type Producer<M>
    = QueueMailboxProducer<Self::Queue<M>, Self::Signal>
  where
    M: Element;
  type Queue<M>
    = crate::api::mailbox::queue_mailbox::LegacyQueueDriver<MpscQueue<SharedBackendHandle<M>, M>>
  where
    M: Element;
  type Signal = TestSignal;

  fn build_mailbox<M>(&self, options: MailboxOptions) -> MailboxPair<Self::Mailbox<M>, Self::Producer<M>>
  where
    M: Element, {
    let capacity = self.resolve_capacity(options);
    let queue =
      crate::api::mailbox::queue_mailbox::LegacyQueueDriver::new(MpscQueue::new(SharedBackendHandle::new(capacity)));
    let signal = TestSignal::default();
    let mailbox = QueueMailbox::new(queue, signal);
    let producer = mailbox.producer();
    (mailbox, producer)
  }
}

#[test]
fn actor_ref_dead_letter_and_metrics_on_queue_v1_embedded_rc_factory() {
  type EmbeddedRuntime = GenericActorRuntime<EmbeddedRcMailboxFactory>;

  let factory = EmbeddedRcMailboxFactory::with_capacity(1);

  let options = MailboxOptions::with_capacity(1);
  let (mut mailbox, producer) = factory.build_mailbox::<PriorityEnvelope<AnyMessage>>(options);
  let priority_ref: PriorityActorRef<AnyMessage, EmbeddedRcMailboxFactory> = PriorityActorRef::new(producer);

  let registry: ArcShared<ActorProcessRegistry<EmbeddedRuntime>> =
    ArcShared::new(ProcessRegistry::new(SystemId::new("embedded-queue-v1"), None));

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

  let path = ActorPath::new().push_child(ActorId(11));
  let handle = ArcShared::new(priority_ref.clone());
  let pid = registry.with_ref(|reg| reg.register_local(path, handle));

  let pid_slot = ArcShared::new(RwLock::new(None));
  let actor_ref: ActorRef<u32, EmbeddedRuntime> =
    ActorRef::new(priority_ref.clone(), pid_slot.clone(), Some(registry.clone()));
  actor_ref.set_pid(pid.clone());

  let first = PriorityEnvelope::with_default_priority(AnyMessage::new(MessageEnvelope::user(0u32)));
  mailbox.try_send_mailbox(first).expect("first enqueue succeeds");

  let result = actor_ref.tell(2u32);
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

#[test]
fn actor_ref_tell_with_priority_queue_v1_emits_dead_letter() {
  type EmbeddedRuntime = GenericActorRuntime<EmbeddedRcMailboxFactory>;

  let factory = EmbeddedRcMailboxFactory::with_capacity(1);
  let options = MailboxOptions::with_capacity(1);
  let (mut mailbox, producer) = factory.build_mailbox::<PriorityEnvelope<AnyMessage>>(options);
  let priority_ref: PriorityActorRef<AnyMessage, EmbeddedRcMailboxFactory> = PriorityActorRef::new(producer);

  let registry: ArcShared<ActorProcessRegistry<EmbeddedRuntime>> =
    ArcShared::new(ProcessRegistry::new(SystemId::new("queue-v1-priority"), None));

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

  let path = ActorPath::new().push_child(ActorId(12));
  let handle = ArcShared::new(priority_ref.clone());
  let pid = registry.with_ref(|reg| reg.register_local(path, handle));

  let pid_slot = ArcShared::new(RwLock::new(None));
  let actor_ref: ActorRef<u32, EmbeddedRuntime> =
    ActorRef::new(priority_ref.clone(), pid_slot.clone(), Some(registry.clone()));
  actor_ref.set_pid(pid.clone());

  let first = PriorityEnvelope::with_default_priority(AnyMessage::new(MessageEnvelope::user(0u32)));
  mailbox.try_send_mailbox(first).expect("first enqueue succeeds");

  let result = actor_ref.tell_with_priority(1u32, 5);
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

#[test]
fn actor_ref_send_system_queue_v1_emits_dead_letter() {
  type EmbeddedRuntime = GenericActorRuntime<EmbeddedRcMailboxFactory>;

  let factory = EmbeddedRcMailboxFactory::with_capacity(1);
  let options = MailboxOptions::with_capacity(1);
  let (mut mailbox, producer) = factory.build_mailbox::<PriorityEnvelope<AnyMessage>>(options);
  let priority_ref: PriorityActorRef<AnyMessage, EmbeddedRcMailboxFactory> = PriorityActorRef::new(producer);

  let registry: ArcShared<ActorProcessRegistry<EmbeddedRuntime>> =
    ArcShared::new(ProcessRegistry::new(SystemId::new("queue-v1-system"), None));

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

  let path = ActorPath::new().push_child(ActorId(13));
  let handle = ArcShared::new(priority_ref.clone());
  let pid = registry.with_ref(|reg| reg.register_local(path, handle));

  let pid_slot = ArcShared::new(RwLock::new(None));
  let actor_ref: ActorRef<u32, EmbeddedRuntime> =
    ActorRef::new(priority_ref.clone(), pid_slot.clone(), Some(registry.clone()));
  actor_ref.set_pid(pid.clone());

  let first = PriorityEnvelope::with_default_priority(AnyMessage::new(MessageEnvelope::user(0u32)));
  mailbox.try_send_mailbox(first).expect("first enqueue succeeds");

  let result = actor_ref.send_system(SystemMessage::Stop);
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
