extern crate alloc;

use alloc::string::String;

use cellex_utils_core_rs::v2::collections::queue::backend::QueueError;

use super::{MailboxError, MailboxOverflowPolicy};

#[test]
fn from_queue_error_maps_variants() {
  let full_message = String::from("queue-full");
  let full_expected = full_message.clone();
  assert_eq!(MailboxError::from_queue_error(QueueError::Full(full_message)), MailboxError::QueueFull {
    policy:    MailboxOverflowPolicy::DropNewest,
    preserved: full_expected,
  },);

  let offer_message = String::from("offer-error");
  let offer_expected = offer_message.clone();
  assert_eq!(MailboxError::from_queue_error(QueueError::OfferError(offer_message)), MailboxError::Internal {
    preserved: offer_expected,
  },);

  let closed_message = String::from("closed");
  let closed_expected = closed_message.clone();
  assert_eq!(MailboxError::from_queue_error(QueueError::Closed(closed_message)), MailboxError::Closed {
    last: Some(closed_expected),
  },);

  assert_eq!(MailboxError::<String>::from_queue_error(QueueError::Disconnected), MailboxError::Disconnected);
  assert_eq!(MailboxError::<String>::from_queue_error(QueueError::WouldBlock), MailboxError::Backpressure);

  let alloc_message = String::from("alloc-error");
  let alloc_expected = alloc_message.clone();
  assert_eq!(MailboxError::from_queue_error(QueueError::AllocError(alloc_message)), MailboxError::ResourceExhausted {
    preserved: alloc_expected,
  },);
}

#[test]
fn from_queue_error_with_policy_applies_hint() {
  let preserved = String::from("drop-oldest");
  let expected = preserved.clone();
  assert_eq!(
    MailboxError::from_queue_error_with_policy(QueueError::Full(preserved), MailboxOverflowPolicy::DropOldest),
    MailboxError::QueueFull { policy: MailboxOverflowPolicy::DropOldest, preserved: expected },
  );
}

#[test]
fn mailbox_error_into_queue_error_roundtrip() {
  let full = String::from("full");
  assert_eq!(
    QueueError::from(MailboxError::QueueFull { policy: MailboxOverflowPolicy::DropNewest, preserved: full.clone() }),
    QueueError::Full(full),
  );

  assert_eq!(QueueError::from(MailboxError::<String>::Disconnected), QueueError::Disconnected);

  let closed = String::from("closed");
  assert_eq!(QueueError::from(MailboxError::Closed { last: Some(closed.clone()) }), QueueError::Closed(closed),);

  assert_eq!(QueueError::from(MailboxError::<String>::Closed { last: None }), QueueError::Disconnected);
  assert_eq!(QueueError::from(MailboxError::<String>::Backpressure), QueueError::WouldBlock);

  let exhausted = String::from("exhausted");
  assert_eq!(
    QueueError::from(MailboxError::ResourceExhausted { preserved: exhausted.clone() }),
    QueueError::AllocError(exhausted),
  );

  let internal = String::from("internal");
  assert_eq!(
    QueueError::from(MailboxError::Internal { preserved: internal.clone() }),
    QueueError::OfferError(internal),
  );
}

#[test]
#[should_panic(expected = "QueueError::Empty cannot be converted into a MailboxError")]
fn from_queue_error_panics_on_empty() {
  // 空キューはエラーではなく呼び出し側で処理すべきケースのため、変換はパニックする。
  let _ = MailboxError::<String>::from_queue_error(QueueError::Empty);
}
