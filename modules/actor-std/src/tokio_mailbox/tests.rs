use std::{
  sync::{Arc, Mutex},
  vec::Vec,
};

use cellex_actor_core_rs::api::{
  mailbox::Mailbox,
  metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
};
use cellex_utils_core_rs::collections::queue::backend::QueueError;

use super::*;

type TestResult<T = ()> = Result<T, String>;

#[derive(Clone)]
struct RecordingSink {
  events: Arc<Mutex<Vec<MetricsEvent>>>,
}

impl RecordingSink {
  fn new(events: Arc<Mutex<Vec<MetricsEvent>>>) -> Self {
    Self { events }
  }
}

impl MetricsSink for RecordingSink {
  fn record(&self, event: MetricsEvent) {
    self.events.lock().unwrap().push(event);
  }
}

async fn run_factory_with_capacity_enforces_bounds() -> TestResult {
  let factory = TokioMailboxFactory;
  let (mailbox, sender) = factory.with_capacity::<u32>(2);

  sender.try_send(1).map_err(|err| format!("first message accepted: {:?}", err))?;
  sender.try_send(2).map_err(|err| format!("second message accepted: {:?}", err))?;
  assert!(matches!(sender.try_send(3), Err(QueueError::Full(3))));
  assert_eq!(mailbox.len().to_usize(), 2);

  let first = mailbox.recv().await.map_err(|err| format!("first message: {:?}", err))?;
  let second = mailbox.recv().await.map_err(|err| format!("second message: {:?}", err))?;

  assert_eq!(first, 1);
  assert_eq!(second, 2);
  assert_eq!(mailbox.len().to_usize(), 0);
  Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn mailbox_with_capacity_enforces_bounds() -> TestResult {
  run_factory_with_capacity_enforces_bounds().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_with_capacity_enforces_bounds_multi_thread_multi_thread() -> TestResult {
  run_factory_with_capacity_enforces_bounds().await
}

async fn run_factory_unbounded_mailbox_accepts_multiple_messages() -> TestResult {
  let factory = TokioMailboxFactory;
  let (mailbox, sender) = factory.unbounded::<u32>();

  for value in 0..32_u32 {
    sender.send(value).map_err(|err| format!("send succeeds: {:?}", err))?;
  }

  assert!(mailbox.capacity().is_limitless());

  for expected in 0..32_u32 {
    let received = mailbox.recv().await.map_err(|err| format!("receive message: {:?}", err))?;
    assert_eq!(received, expected);
  }

  assert_eq!(mailbox.len().to_usize(), 0);
  Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn mailbox_unbounded_accepts_multiple_messages() -> TestResult {
  run_factory_unbounded_mailbox_accepts_multiple_messages().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_unbounded_accepts_multiple_messages_multi_thread() -> TestResult {
  run_factory_unbounded_mailbox_accepts_multiple_messages().await
}

#[tokio::test(flavor = "current_thread")]
async fn mailbox_emits_growth_metric() -> TestResult {
  let factory = TokioMailboxFactory;
  let (mut mailbox, mut sender) = factory.unbounded::<u32>();

  let events = Arc::new(Mutex::new(Vec::new()));
  let sink = MetricsSinkShared::new(RecordingSink::new(events.clone()));

  mailbox.set_metrics_sink(Some(sink.clone()));
  sender.set_metrics_sink(Some(sink.clone()));

  sender.try_send(1u32).map_err(|err| format!("first message should succeed: {err:?}"))?;

  // take out the queue to avoid borrow issues? not needed.
  let recorded = events.lock().unwrap().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxGrewTo { capacity } if *capacity >= 1)),
    "expected MailboxGrewTo event, recorded: {recorded:?}"
  );

  Ok(())
}
