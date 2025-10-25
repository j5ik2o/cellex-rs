extern crate alloc;

use alloc::vec::Vec;

use cellex_utils_core_rs::{
  collections::queue::QueueError,
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
};

use super::*;
use crate::api::metrics::{MetricsEvent, MetricsSink, MetricsSinkShared};

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
fn records_drop_oldest_metric() {
  let queue = QueueRwCompat::bounded(1, OverflowPolicy::DropOldest);
  let (metrics, events) = make_sink();
  queue.set_metrics_sink(Some(metrics));

  queue.offer(1u32).expect("first offer succeeds");
  queue.offer(2u32).expect("second offer drops oldest");

  let recorded = collected(&events);
  assert!(recorded.contains(&MetricsEvent::MailboxDroppedOldest { count: 1 }));
}

#[test]
fn records_drop_newest_metric() {
  let queue = QueueRwCompat::bounded(1, OverflowPolicy::DropNewest);
  let (metrics, events) = make_sink();
  queue.set_metrics_sink(Some(metrics));

  queue.offer(1u32).expect("first offer succeeds");
  let Err(QueueError::Full(value)) = queue.offer(2u32) else {
    panic!("expected QueueError::Full");
  };
  assert_eq!(value, 2u32);

  let recorded = collected(&events);
  assert!(recorded.contains(&MetricsEvent::MailboxDroppedNewest { count: 1 }));
}

#[test]
fn records_grew_to_metric() {
  let queue = QueueRwCompat::bounded(1, OverflowPolicy::Grow);
  let (metrics, events) = make_sink();
  queue.set_metrics_sink(Some(metrics));

  queue.offer(1u32).expect("first offer succeeds");
  queue.offer(2u32).expect("second offer triggers growth");

  let recorded = collected(&events);
  assert!(recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxGrewTo { capacity } if *capacity >= 2)));
}
