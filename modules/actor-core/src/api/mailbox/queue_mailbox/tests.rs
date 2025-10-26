extern crate alloc;

use alloc::vec::Vec;
use core::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use cellex_utils_core_rs::{
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared},
  v2::collections::queue::backend::{OfferOutcome, OverflowPolicy},
  Element, QueueError, QueueSize,
};
use futures::task::noop_waker_ref;

use super::{MailboxQueueDriver, QueueMailbox, QueuePollOutcome};
use crate::api::{
  mailbox::{queue_mailbox::SyncQueueDriver, Mailbox, MailboxError, MailboxOverflowPolicy, MailboxProducer},
  metrics::{MetricsEvent, MetricsSink, MetricsSinkShared},
  test_support::TestSignal,
};

struct ErrorDriver<M> {
  overflow_policy: Option<MailboxOverflowPolicy>,
  error:           fn(M) -> QueueError<M>,
}

impl<M> Copy for ErrorDriver<M> {}

impl<M> Clone for ErrorDriver<M> {
  fn clone(&self) -> Self {
    Self { overflow_policy: self.overflow_policy, error: self.error }
  }
}

impl<M> ErrorDriver<M>
where
  M: Element,
{
  const fn new(overflow_policy: Option<MailboxOverflowPolicy>, error: fn(M) -> QueueError<M>) -> Self {
    Self { overflow_policy, error }
  }
}

impl<M> MailboxQueueDriver<M> for ErrorDriver<M>
where
  M: Element,
{
  fn len(&self) -> QueueSize {
    QueueSize::limitless()
  }

  fn capacity(&self) -> QueueSize {
    QueueSize::limitless()
  }

  fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>> {
    Err((self.error)(message))
  }

  fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>> {
    Ok(QueuePollOutcome::Empty)
  }

  fn close(&self) -> Result<Option<M>, QueueError<M>> {
    Ok(None)
  }

  fn set_metrics_sink(&self, _sink: Option<MetricsSinkShared>) {}

  fn overflow_policy(&self) -> Option<MailboxOverflowPolicy> {
    self.overflow_policy
  }
}

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

fn make_metrics_sink() -> (MetricsSinkShared, ArcShared<SpinSyncMutex<Vec<MetricsEvent>>>) {
  let (sink, events) = RecordingSink::new();
  (MetricsSinkShared::new(sink), events)
}

fn queue_full<T: Element>(message: T) -> QueueError<T> {
  QueueError::Full(message)
}

fn queue_would_block<T: Element>(message: T) -> QueueError<T> {
  let _ = message;
  QueueError::WouldBlock
}

fn queue_alloc_error<T: Element>(message: T) -> QueueError<T> {
  QueueError::AllocError(message)
}

fn queue_offer_error<T: Element>(message: T) -> QueueError<T> {
  QueueError::OfferError(message)
}

#[test]
fn sync_queue_drop_newest_returns_queue_full_with_policy() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::DropNewest);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  mailbox.try_send_mailbox(1u32).expect("first message enqueued");
  let err = mailbox.try_send_mailbox(2u32).expect_err("second message should fail");

  match err {
    | MailboxError::QueueFull { policy, preserved } => {
      assert_eq!(policy, MailboxOverflowPolicy::DropNewest);
      assert_eq!(preserved, 2u32);
    },
    | other => panic!("unexpected error: {other:?}"),
  }
}

#[test]
fn sync_queue_drop_oldest_keeps_mailbox_usable() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::DropOldest);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  mailbox.try_send_mailbox(1u32).expect("first enqueue succeeds");
  mailbox.try_send_mailbox(2u32).expect("drop oldest behaves as success");

  let received = mailbox.core.try_dequeue_mailbox::<u32>().expect("dequeue result");
  assert_eq!(received, Some(2u32));
}

#[test]
fn sync_queue_overflow_policy_propagates_through_queue_error_path() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::Block);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  mailbox.try_send_mailbox(1u32).expect("first enqueue succeeds");

  let error = mailbox.try_send_mailbox(2u32).expect_err("queue reports blocking policy as full");
  let MailboxError::QueueFull { policy, preserved } = error else {
    panic!("expected QueueFull error");
  };
  assert_eq!(policy, MailboxOverflowPolicy::Block);
  assert_eq!(preserved, 2u32);
}

#[test]
fn queue_mailbox_reports_drop_oldest_policy_on_queue_error() {
  let driver = ErrorDriver::new(Some(MailboxOverflowPolicy::DropOldest), queue_full::<u32>);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  let error = mailbox.try_send_mailbox(1u32).expect_err("expected queue full error");

  match error {
    | MailboxError::QueueFull { policy, preserved } => {
      assert_eq!(policy, MailboxOverflowPolicy::DropOldest);
      assert_eq!(preserved, 1u32);
    },
    | other => panic!("unexpected error: {other:?}"),
  }
  assert!(!mailbox.core.closed().get(), "drop oldest must not close the mailbox");
}

#[test]
fn queue_mailbox_backpressure_maps_to_mailbox_error() {
  let driver = ErrorDriver::new(None, queue_would_block::<u32>);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  let error = mailbox.try_send_mailbox(7u32).expect_err("expected backpressure error");
  assert!(matches!(error, MailboxError::Backpressure));
  assert!(!mailbox.core.closed().get(), "backpressure must keep mailbox open");
}

#[test]
fn queue_mailbox_resource_exhausted_maps_to_mailbox_error() {
  let driver = ErrorDriver::new(None, queue_alloc_error::<u32>);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  let error = mailbox.try_send_mailbox(9u32).expect_err("expected resource exhaustion error");

  match error {
    | MailboxError::ResourceExhausted { preserved } => assert_eq!(preserved, 9u32),
    | other => panic!("unexpected error: {other:?}"),
  }
  assert!(!mailbox.core.closed().get(), "resource exhaustion must leave the mailbox open");
}

#[test]
fn queue_mailbox_internal_error_preserves_message() {
  let driver = ErrorDriver::new(None, queue_offer_error::<u32>);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  let error = mailbox.try_send_mailbox(11u32).expect_err("expected internal error");

  match error {
    | MailboxError::Internal { preserved } => assert_eq!(preserved, 11u32),
    | other => panic!("unexpected error: {other:?}"),
  }
  assert!(!mailbox.core.closed().get(), "internal errors are recoverable");
}

#[test]
fn producer_reports_dropped_oldest_outcome() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::DropOldest);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);
  let producer = mailbox.producer();

  producer.try_send_with_outcome(1u32).expect("first enqueue succeeds");
  let outcome = producer.try_send_with_outcome(2u32).expect("drop oldest should still be a success");

  match outcome {
    | OfferOutcome::DroppedOldest { count } => assert_eq!(count, 1),
    | other => panic!("unexpected outcome: {other:?}"),
  }
}

#[test]
fn producer_reports_enqueued_outcome() {
  let driver = SyncQueueDriver::bounded(2, OverflowPolicy::Block);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);
  let producer = mailbox.producer();

  let outcome = producer.try_send_with_outcome(42u32).expect("enqueue succeeds");

  assert!(matches!(outcome, OfferOutcome::Enqueued));
}

#[test]
fn receiver_pending_until_message_arrives() {
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(SyncQueueDriver::bounded(1, OverflowPolicy::Block), signal);
  let mut recv_future = mailbox.recv();

  let waker = noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  let poll = unsafe { Pin::new_unchecked(&mut recv_future) }.poll(&mut cx);
  assert!(matches!(poll, Poll::Pending), "expected pending poll, got {poll:?}");

  mailbox.try_send_mailbox(99u32).expect("enqueue succeeds");

  let result = unsafe { Pin::new_unchecked(&mut recv_future) }.poll(&mut cx);
  match result {
    | Poll::Ready(Ok(message)) => assert_eq!(message, 99u32),
    | other => panic!("unexpected poll result: {other:?}"),
  }
}

#[test]
fn producer_records_drop_oldest_metric() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::DropOldest);
  let signal = TestSignal::default();
  let mut mailbox = QueueMailbox::new(driver, signal);
  let (sink, events) = make_metrics_sink();
  Mailbox::set_metrics_sink(&mut mailbox, Some(sink.clone()));
  let mut producer = mailbox.producer();
  MailboxProducer::set_metrics_sink(&mut producer, Some(sink));

  producer.try_send_with_outcome(1u32).expect("first enqueue succeeds");
  producer.try_send_with_outcome(2u32).expect("drop oldest succeeds");

  let recorded = events.lock().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxDroppedOldest { count: 1 })),
    "expected MailboxDroppedOldest event but got {recorded:?}"
  );
}

#[test]
fn producer_records_drop_newest_metric_and_error() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::DropNewest);
  let signal = TestSignal::default();
  let mut mailbox = QueueMailbox::new(driver, signal);
  let (sink, events) = make_metrics_sink();
  Mailbox::set_metrics_sink(&mut mailbox, Some(sink.clone()));
  let mut producer = mailbox.producer();
  MailboxProducer::set_metrics_sink(&mut producer, Some(sink));

  producer.try_send_mailbox(1u32).expect("first enqueue succeeds");
  let error = producer.try_send_mailbox(2u32).expect_err("second enqueue should fail");

  match error {
    | MailboxError::QueueFull { policy, preserved } => {
      assert_eq!(policy, MailboxOverflowPolicy::DropNewest);
      assert_eq!(preserved, 2u32);
    },
    | other => panic!("unexpected error: {other:?}"),
  }

  let recorded = events.lock().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxDroppedNewest { count: 1 })),
    "expected MailboxDroppedNewest event but got {recorded:?}"
  );
}

#[test]
fn producer_records_grow_metric() {
  let driver = SyncQueueDriver::bounded(1, OverflowPolicy::Grow);
  let signal = TestSignal::default();
  let mut mailbox = QueueMailbox::new(driver, signal);
  let (sink, events) = make_metrics_sink();
  Mailbox::set_metrics_sink(&mut mailbox, Some(sink.clone()));
  let mut producer = mailbox.producer();
  MailboxProducer::set_metrics_sink(&mut producer, Some(sink));

  producer.try_send_with_outcome(1u32).expect("first enqueue succeeds");
  let outcome = producer.try_send_with_outcome(2u32).expect("growth enqueue succeeds");

  match outcome {
    | OfferOutcome::GrewTo { capacity } => assert!(capacity >= 2, "unexpected capacity {capacity}"),
    | other => panic!("expected growth outcome, got {other:?}"),
  }

  let recorded = events.lock().clone();
  assert!(
    recorded.iter().any(|event| matches!(event, MetricsEvent::MailboxGrewTo { capacity } if *capacity >= 2)),
    "expected MailboxGrewTo event but got {recorded:?}"
  );
}
