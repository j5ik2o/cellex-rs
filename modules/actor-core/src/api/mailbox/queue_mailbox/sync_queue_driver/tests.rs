extern crate alloc;

use alloc::vec::Vec;

use cellex_utils_core_rs::{
  collections::queue::QueueError,
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
  v2::collections::queue::backend::OverflowPolicy,
};

use super::{QueuePollOutcome, SyncQueueDriver};
use crate::api::{
  mailbox::queue_mailbox::MailboxQueueDriver,
  metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
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

fn make_sink() -> (MetricsSinkShared, ArcShared<SpinSyncMutex<Vec<MetricsEvent>>>) {
  let (sink, events) = RecordingSink::new();
  (MetricsSinkShared::new(sink), events)
}

fn collected(events: &ArcShared<SpinSyncMutex<Vec<MetricsEvent>>>) -> Vec<MetricsEvent> {
  events.lock().clone()
}

#[test]
fn offer_drop_oldest_records_metric() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::DropOldest);
  let (metrics, events) = make_sink();
  driver.set_metrics_sink(Some(metrics));

  driver.offer(1u32).expect("first offer succeeds");
  driver.offer(2u32).expect("second offer drops oldest");

  let recorded = collected(&events);
  assert!(recorded.contains(&MetricsEvent::MailboxDroppedOldest { count: 1 }));
}

#[test]
fn offer_drop_newest_returns_full_error() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::DropNewest);
  let (metrics, events) = make_sink();
  driver.set_metrics_sink(Some(metrics));

  driver.offer(1u32).expect("first offer succeeds");
  let Err(QueueError::Full(message)) = driver.offer(2u32) else {
    panic!("expected QueueError::Full");
  };
  assert_eq!(message, 2u32);

  let recorded = collected(&events);
  assert!(recorded.contains(&MetricsEvent::MailboxDroppedNewest { count: 1 }));
}

#[test]
fn offer_grow_records_metric() {
  let driver = SyncQueueDriver::unbounded();
  let (metrics, events) = make_sink();
  driver.set_metrics_sink(Some(metrics));

  driver.offer(1u32).expect("offer triggers growth");

  let recorded = collected(&events);
  assert!(recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxGrewTo { capacity } if *capacity >= 1)));
}

#[test]
fn poll_yields_message_and_empty() {
  let driver = SyncQueueDriver::bounded(2, OverflowPolicy::Block);
  driver.offer(42u32).expect("first offer succeeds");

  match driver.poll().expect("poll succeeds") {
    | QueuePollOutcome::Message(value) => assert_eq!(value, 42u32),
    | QueuePollOutcome::Empty => panic!("expected message, queue was empty"),
    | QueuePollOutcome::Pending => panic!("expected message, queue reported pending"),
    | QueuePollOutcome::Disconnected => panic!("expected message, queue disconnected"),
    | QueuePollOutcome::Closed(_) => panic!("expected message, queue closed"),
    | QueuePollOutcome::Err(_) => panic!("expected message, queue returned error"),
  }

  match driver.poll().expect("poll succeeds") {
    | QueuePollOutcome::Empty => {},
    | QueuePollOutcome::Pending => panic!("expected empty, queue reported pending"),
    | QueuePollOutcome::Message(_) => panic!("expected empty, message remained"),
    | QueuePollOutcome::Disconnected => panic!("expected empty, queue disconnected"),
    | QueuePollOutcome::Closed(_) => panic!("expected empty, queue closed"),
    | QueuePollOutcome::Err(_) => panic!("expected empty, queue returned error"),
  }
}
