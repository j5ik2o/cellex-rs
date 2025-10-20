use cellex_utils_std_rs::QueueError;

use super::*;

type TestResult<T = ()> = Result<T, String>;

async fn run_runtime_with_capacity_enforces_bounds() -> TestResult {
  let factory = TokioMailboxRuntime;
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
async fn runtime_with_capacity_enforces_bounds() -> TestResult {
  run_runtime_with_capacity_enforces_bounds().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_with_capacity_enforces_bounds_multi_thread() -> TestResult {
  run_runtime_with_capacity_enforces_bounds().await
}

async fn run_runtime_unbounded_mailbox_accepts_multiple_messages() -> TestResult {
  let factory = TokioMailboxRuntime;
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
async fn runtime_unbounded_mailbox_accepts_multiple_messages() -> TestResult {
  run_runtime_unbounded_mailbox_accepts_multiple_messages().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_unbounded_mailbox_accepts_multiple_messages_multi_thread() -> TestResult {
  run_runtime_unbounded_mailbox_accepts_multiple_messages().await
}
