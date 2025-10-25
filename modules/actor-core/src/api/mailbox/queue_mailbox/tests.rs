#![cfg(feature = "queue-v2")]

use cellex_utils_core_rs::v2::collections::queue::backend::OverflowPolicy;

use super::QueueMailbox;
use crate::api::{
  mailbox::{queue_mailbox::SyncQueueDriver, MailboxError, MailboxOverflowPolicy},
  test_support::TestSignal,
};

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
