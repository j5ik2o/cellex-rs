use super::*;
use cellex_utils_std_rs::{QueueSize, DEFAULT_PRIORITY};

#[test]
fn priority_runtime_orders_messages() {
  let factory = TokioPriorityMailboxRuntime::default();
  let (mailbox, sender) = factory.mailbox::<u32>(MailboxOptions::default());

  sender
    .send_with_priority(10, DEFAULT_PRIORITY)
    .expect("send low priority");
  sender
    .send_control_with_priority(99, DEFAULT_PRIORITY + 7)
    .expect("send high priority");
  sender
    .send_control_with_priority(20, DEFAULT_PRIORITY + 3)
    .expect("send medium priority");

  let first = mailbox.inner().queue().poll().unwrap().unwrap();
  let second = mailbox.inner().queue().poll().unwrap().unwrap();
  let third = mailbox.inner().queue().poll().unwrap().unwrap();

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 7));
  assert_eq!(second.into_parts(), (20, DEFAULT_PRIORITY + 3));
  assert_eq!(third.into_parts(), (10, DEFAULT_PRIORITY));
}

#[test]
fn priority_sender_defaults_work() {
  let factory = TokioPriorityMailboxRuntime::new(4).with_regular_capacity(4);
  let (mailbox, sender) = factory.mailbox::<u8>(MailboxOptions::default());

  sender
    .send(PriorityEnvelope::with_default_priority(5))
    .expect("send default priority");

  let envelope = mailbox.inner().queue().poll().unwrap().unwrap();
  let (_, priority) = envelope.into_parts();
  assert_eq!(priority, DEFAULT_PRIORITY);
}

#[test]
fn control_queue_preempts_regular_messages() {
  let factory = TokioPriorityMailboxRuntime::default();
  let (mailbox, sender) = factory.mailbox::<u32>(MailboxOptions::default());

  sender
    .send_with_priority(1, DEFAULT_PRIORITY)
    .expect("enqueue regular message");
  sender
    .send_control_with_priority(99, DEFAULT_PRIORITY + 5)
    .expect("enqueue control message");

  let first = mailbox.inner().queue().poll().unwrap().unwrap();
  let second = mailbox.inner().queue().poll().unwrap().unwrap();

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 5));
  assert_eq!(second.into_parts(), (1, DEFAULT_PRIORITY));
}

#[test]
fn priority_mailbox_capacity_split() {
  let factory = TokioPriorityMailboxRuntime::default();
  let options = MailboxOptions::with_capacities(QueueSize::limited(2), QueueSize::limited(2));
  let (mailbox, sender) = factory.mailbox::<u8>(options);

  assert!(!mailbox.capacity().is_limitless());

  sender
    .send_control_with_priority(1, DEFAULT_PRIORITY + 2)
    .expect("control enqueue");
  sender.send_with_priority(2, DEFAULT_PRIORITY).expect("regular enqueue");
  sender
    .send_with_priority(3, DEFAULT_PRIORITY)
    .expect("second regular enqueue");

  let err = sender
    .try_send_with_priority(4, DEFAULT_PRIORITY)
    .expect_err("regular capacity reached");
  assert!(matches!(&*err, QueueError::Full(_)));
}
