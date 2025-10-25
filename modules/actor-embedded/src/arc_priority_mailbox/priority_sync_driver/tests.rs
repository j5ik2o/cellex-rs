extern crate alloc;

use alloc::vec::Vec;

use cellex_actor_core_rs::{
  api::{
    mailbox::MailboxOverflowPolicy,
    metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
  },
  shared::mailbox::messages::PriorityEnvelope,
};
use cellex_utils_core_rs::sync::{sync_mutex_like::SpinSyncMutex, ArcShared};

use super::*;

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

#[test]
fn control_lane_has_priority_over_regular() {
  let driver = PrioritySyncQueueDriver::new(2, 1, 1);
  driver.offer(PriorityEnvelope::new(1u32, 0)).expect("regular offer succeeds");
  driver.offer(PriorityEnvelope::control(10u32, 1)).expect("control offer succeeds");

  match driver.poll().expect("poll succeeds") {
    | QueuePollOutcome::Message(envelope) => {
      assert!(envelope.is_control());
      assert_eq!(*envelope.message(), 10u32);
    },
    | _ => panic!("expected control message"),
  }
}

#[test]
fn metrics_sink_broadcasts_to_all_lanes() {
  let driver = PrioritySyncQueueDriver::new(1, 0, 0);
  let (sink, events) = make_sink();
  driver.set_metrics_sink(Some(sink));

  driver.offer(PriorityEnvelope::with_default_priority(1u32)).expect("offer should succeed and emit growth event");

  let recorded = events.lock().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxGrewTo { capacity } if *capacity >= 1)),
    "metrics not captured: {recorded:?}"
  );
}

#[test]
fn len_and_capacity_aggregate_across_lanes() {
  let driver = PrioritySyncQueueDriver::new(2, 2, 3);
  assert_eq!(driver.capacity().to_usize(), 7);

  driver.offer(PriorityEnvelope::control(1u32, 1)).unwrap();
  driver.offer(PriorityEnvelope::new(2u32, 0)).unwrap();

  assert_eq!(driver.len().to_usize(), 2);
}

#[test]
fn overflow_policy_reflects_blocking_behavior() {
  let driver: PrioritySyncQueueDriver<u32> = PrioritySyncQueueDriver::new(1, 1, 1);

  assert_eq!(driver.overflow_policy(), Some(MailboxOverflowPolicy::Block));
}
