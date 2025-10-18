use core::sync::atomic::{AtomicBool, Ordering};

use cellex_utils_embedded_rs::{QueueSize, DEFAULT_PRIORITY};
use critical_section::{Impl, RawRestoreState};

use super::*;

fn prepare() {
  init_critical_section();
}

struct TestCriticalSection;

static CS_LOCK: AtomicBool = AtomicBool::new(false);
static CS_INIT: AtomicBool = AtomicBool::new(false);

unsafe impl Impl for TestCriticalSection {
  unsafe fn acquire() -> RawRestoreState {
    while CS_LOCK.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {}
    ()
  }

  unsafe fn release(_: RawRestoreState) {
    CS_LOCK.store(false, Ordering::SeqCst);
  }
}

fn init_critical_section() {
  if CS_INIT.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
    critical_section::set_impl!(TestCriticalSection);
  }
}

#[test]
fn priority_mailbox_orders_messages_by_priority() {
  prepare();
  let factory = ArcPriorityMailboxRuntime::<CriticalSectionRawMutex>::default();
  let (mailbox, sender) = factory.mailbox::<u8>(MailboxOptions::default());

  sender.try_send_with_priority(10, DEFAULT_PRIORITY).expect("low priority");
  sender.try_send_control_with_priority(99, DEFAULT_PRIORITY + 7).expect("high priority");
  sender.try_send_control_with_priority(20, DEFAULT_PRIORITY + 3).expect("medium priority");

  let first = mailbox.inner().queue().poll().unwrap().unwrap();
  let second = mailbox.inner().queue().poll().unwrap().unwrap();
  let third = mailbox.inner().queue().poll().unwrap().unwrap();

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 7));
  assert_eq!(second.into_parts(), (20, DEFAULT_PRIORITY + 3));
  assert_eq!(third.into_parts(), (10, DEFAULT_PRIORITY));
}

#[test]
fn priority_mailbox_capacity_split() {
  prepare();
  let factory = ArcPriorityMailboxRuntime::<CriticalSectionRawMutex>::default();
  let options = MailboxOptions::with_capacities(QueueSize::limited(2), QueueSize::limited(2));
  let (mailbox, sender) = factory.mailbox::<u8>(options);

  assert!(!mailbox.capacity().is_limitless());

  sender.try_send_control_with_priority(1, DEFAULT_PRIORITY + 3).expect("control enqueue");
  sender.try_send_with_priority(2, DEFAULT_PRIORITY).expect("regular enqueue");
  sender.try_send_with_priority(3, DEFAULT_PRIORITY).expect("second regular enqueue");

  let err = sender.try_send_with_priority(4, DEFAULT_PRIORITY).expect_err("regular capacity reached");
  assert!(matches!(err, QueueError::Full(_)));
}

#[test]
fn control_queue_preempts_regular_messages() {
  prepare();
  let factory = ArcPriorityMailboxRuntime::<CriticalSectionRawMutex>::default();
  let (mailbox, sender) = factory.mailbox::<u32>(MailboxOptions::default());

  sender.try_send_with_priority(1, DEFAULT_PRIORITY).expect("regular message");
  sender.try_send_control_with_priority(99, DEFAULT_PRIORITY + 5).expect("control message");

  let first = mailbox.inner().queue().poll().unwrap().unwrap();
  let second = mailbox.inner().queue().poll().unwrap().unwrap();

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 5));
  assert_eq!(second.into_parts(), (1, DEFAULT_PRIORITY));
}
