use super::*;
use cellex_utils_std_rs::QueueError;

async fn run_runtime_with_capacity_enforces_bounds() {
  let factory = TokioMailboxRuntime;
  let (mailbox, sender) = factory.with_capacity::<u32>(2);

  sender.try_send(1).expect("first message accepted");
  sender.try_send(2).expect("second message accepted");
  assert!(matches!(sender.try_send(3), Err(QueueError::Full(3))));
  assert_eq!(mailbox.len().to_usize(), 2);

  let first = mailbox.recv().await.expect("first message");
  let second = mailbox.recv().await.expect("second message");

  assert_eq!(first, 1);
  assert_eq!(second, 2);
  assert_eq!(mailbox.len().to_usize(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn runtime_with_capacity_enforces_bounds() {
  run_runtime_with_capacity_enforces_bounds().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_with_capacity_enforces_bounds_multi_thread() {
  run_runtime_with_capacity_enforces_bounds().await;
}

async fn run_runtime_unbounded_mailbox_accepts_multiple_messages() {
  let factory = TokioMailboxRuntime;
  let (mailbox, sender) = factory.unbounded::<u32>();

  for value in 0..32_u32 {
    sender.send(value).expect("send succeeds");
  }

  assert!(mailbox.capacity().is_limitless());

  for expected in 0..32_u32 {
    let received = mailbox.recv().await.expect("receive message");
    assert_eq!(received, expected);
  }

  assert_eq!(mailbox.len().to_usize(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn runtime_unbounded_mailbox_accepts_multiple_messages() {
  run_runtime_unbounded_mailbox_accepts_multiple_messages().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_unbounded_mailbox_accepts_multiple_messages_multi_thread() {
  run_runtime_unbounded_mailbox_accepts_multiple_messages().await;
}
