use super::*;
use cellex_utils_std_rs::{QueueSize, DEFAULT_PRIORITY};

async fn run_priority_runtime_orders_messages() {
  let factory = TokioPriorityMailboxRuntime::default();
  let (mailbox, sender) = factory.mailbox::<u32>(MailboxOptions::default());

  sender
    .send_with_priority(10, DEFAULT_PRIORITY)
    .await
    .expect("send low priority");
  sender
    .send_control_with_priority(99, DEFAULT_PRIORITY + 7)
    .await
    .expect("send high priority");
  sender
    .send_control_with_priority(20, DEFAULT_PRIORITY + 3)
    .await
    .expect("send medium priority");

  tokio::task::yield_now().await;

  let first = mailbox.recv().await.expect("first message");
  let second = mailbox.recv().await.expect("second message");
  let third = mailbox.recv().await.expect("third message");

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 7));
  assert_eq!(second.into_parts(), (20, DEFAULT_PRIORITY + 3));
  assert_eq!(third.into_parts(), (10, DEFAULT_PRIORITY));
}

#[tokio::test(flavor = "current_thread")]
async fn priority_runtime_orders_messages() {
  run_priority_runtime_orders_messages().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn priority_runtime_orders_messages_multi_thread() {
  run_priority_runtime_orders_messages().await;
}

async fn run_priority_sender_defaults_work() {
  let factory = TokioPriorityMailboxRuntime::new(4).with_regular_capacity(4);
  let (mailbox, sender) = factory.mailbox::<u8>(MailboxOptions::default());

  sender
    .send(PriorityEnvelope::with_default_priority(5))
    .await
    .expect("send default priority");

  let envelope = mailbox.recv().await.expect("receive envelope");
  let (_, priority) = envelope.into_parts();
  assert_eq!(priority, DEFAULT_PRIORITY);
}

#[tokio::test(flavor = "current_thread")]
async fn priority_sender_defaults_work() {
  run_priority_sender_defaults_work().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn priority_sender_defaults_work_multi_thread() {
  run_priority_sender_defaults_work().await;
}

async fn run_control_queue_preempts_regular_messages() {
  let factory = TokioPriorityMailboxRuntime::default();
  let (mailbox, sender) = factory.mailbox::<u32>(MailboxOptions::default());

  sender
    .send_with_priority(1, DEFAULT_PRIORITY)
    .await
    .expect("enqueue regular message");
  sender
    .send_control_with_priority(99, DEFAULT_PRIORITY + 5)
    .await
    .expect("enqueue control message");

  let first = mailbox.recv().await.expect("first message");
  let second = mailbox.recv().await.expect("second message");

  assert_eq!(first.into_parts(), (99, DEFAULT_PRIORITY + 5));
  assert_eq!(second.into_parts(), (1, DEFAULT_PRIORITY));
}

#[tokio::test(flavor = "current_thread")]
async fn control_queue_preempts_regular_messages() {
  run_control_queue_preempts_regular_messages().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn control_queue_preempts_regular_messages_multi_thread() {
  run_control_queue_preempts_regular_messages().await;
}

async fn run_priority_mailbox_capacity_split() {
  let factory = TokioPriorityMailboxRuntime::default();
  let options = MailboxOptions::with_capacities(QueueSize::limited(2), QueueSize::limited(2));
  let (mailbox, sender) = factory.mailbox::<u8>(options);

  assert!(!mailbox.capacity().is_limitless());

  sender
    .send_control_with_priority(1, DEFAULT_PRIORITY + 2)
    .await
    .expect("control enqueue");
  sender
    .send_with_priority(2, DEFAULT_PRIORITY)
    .await
    .expect("regular enqueue");
  sender
    .send_with_priority(3, DEFAULT_PRIORITY)
    .await
    .expect("second regular enqueue");

  let err = sender
    .try_send_with_priority(4, DEFAULT_PRIORITY)
    .expect_err("regular capacity reached");
  assert!(matches!(&*err, QueueError::Full(_)));
}

#[tokio::test(flavor = "current_thread")]
async fn priority_mailbox_capacity_split() {
  run_priority_mailbox_capacity_split().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn priority_mailbox_capacity_split_multi_thread() {
  run_priority_mailbox_capacity_split().await;
}
