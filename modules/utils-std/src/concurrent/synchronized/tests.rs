use super::{Synchronized, SynchronizedRw};

#[tokio::test]
async fn synchronized_mutex_read_write() {
  let sync = Synchronized::new(0_u32);

  let read_val = sync.read(|guard| **guard).await;
  assert_eq!(read_val, 0);

  sync
    .write(|guard| {
      **guard = 5;
    })
    .await;

  let result = {
    let guard = sync.lock().await;
    let guard = guard.into_inner();
    *guard
  };

  assert_eq!(result, 5);
}

#[tokio::test]
async fn synchronized_rw_readers_and_writer() {
  let sync = SynchronizedRw::new(vec![1, 2, 3]);

  let sum = sync.read(|guard| guard.iter().copied().sum::<i32>()).await;
  assert_eq!(sum, 6);

  sync
    .write(|guard| {
      guard.push(4);
    })
    .await;

  let len = {
    let guard = sync.read_guard().await;
    guard.len()
  };
  assert_eq!(len, 4);
}
