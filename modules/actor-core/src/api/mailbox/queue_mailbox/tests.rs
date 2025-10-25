#![cfg(feature = "queue-v2")]

use cellex_utils_core_rs::{
  collections::queue::QueueError,
  v2::collections::queue::backend::{OfferOutcome, OverflowPolicy},
  Element, QueueSize,
};

use super::{MailboxQueueDriver, QueueMailbox, QueuePollOutcome};
use crate::api::{
  mailbox::{queue_mailbox::SyncQueueDriver, MailboxError, MailboxOverflowPolicy},
  metrics::MetricsSinkShared,
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

#[cfg(feature = "queue-v2")]
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

#[cfg(feature = "queue-v2")]
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

#[cfg(feature = "queue-v2")]
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

#[cfg(feature = "queue-v2")]
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

#[cfg(feature = "queue-v2")]
#[test]
fn queue_mailbox_backpressure_maps_to_mailbox_error() {
  let driver = ErrorDriver::new(None, queue_would_block::<u32>);
  let signal = TestSignal::default();
  let mailbox = QueueMailbox::new(driver, signal);

  let error = mailbox.try_send_mailbox(7u32).expect_err("expected backpressure error");
  assert!(matches!(error, MailboxError::Backpressure));
  assert!(!mailbox.core.closed().get(), "backpressure must keep mailbox open");
}

#[cfg(feature = "queue-v2")]
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

#[cfg(feature = "queue-v2")]
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
