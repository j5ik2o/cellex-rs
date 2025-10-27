use futures::{executor::LocalPool, task::LocalSpawnExt};

use super::*;

const CAPACITY: usize = 4;

#[test]
fn offer_and_poll_operates() {
  let queue = make_embassy_mpsc_queue::<u32, CAPACITY>();

  let mut pool = LocalPool::new();
  let spawner = pool.spawner();
  let producer = queue.clone();

  spawner
    .spawn_local(async move {
      producer.offer(7).await.unwrap();
    })
    .unwrap();

  let value = pool.run_until(async { queue.poll().await.unwrap() });
  assert_eq!(value, 7);
}

#[test]
fn offer_blocks_until_capacity_available() {
  const CAP: usize = 1;
  let queue = make_embassy_mpsc_queue::<u8, CAP>();

  let mut pool = LocalPool::new();
  let spawner = pool.spawner();
  let producer = queue.clone();

  spawner
    .spawn_local(async move {
      producer.offer(1).await.unwrap();
      producer.offer(2).await.unwrap();
    })
    .unwrap();

  let first = pool.run_until(async { queue.poll().await.unwrap() });
  assert_eq!(first, 1);

  let second = pool.run_until(async { queue.poll().await.unwrap() });
  assert_eq!(second, 2);
}

#[test]
fn close_rejects_subsequent_operations() {
  let queue = make_embassy_mpsc_queue::<&'static str, CAPACITY>();

  let mut pool = LocalPool::new();
  let close_queue = queue.clone();

  pool
    .spawner()
    .spawn_local(async move {
      close_queue.offer("hello").await.unwrap();
      close_queue.close().await.unwrap();
    })
    .unwrap();

  let value = pool.run_until(async { queue.poll().await.unwrap() });
  assert_eq!(value, "hello");

  let err = pool.run_until(async { queue.poll().await.err().unwrap() });
  assert_eq!(err, QueueError::Disconnected);
  let err = pool.run_until(async { queue.offer("world").await.err().unwrap() });
  assert!(matches!(err, QueueError::Closed(value) if value == "world"));
}
