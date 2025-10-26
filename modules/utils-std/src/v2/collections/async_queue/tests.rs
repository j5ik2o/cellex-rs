use std::time::Duration;

use cellex_utils_core_rs::v2::collections::queue::backend::QueueError;
use tokio::{pin, time::sleep};

use super::{make_tokio_mpsc_queue, TokioMpscQueue};

#[tokio::test(flavor = "multi_thread")]
async fn offer_and_poll_roundtrip() {
  let queue: TokioMpscQueue<i32> = make_tokio_mpsc_queue(4);

  assert!(queue.offer(10).await.is_ok());
  assert_eq!(queue.len().await, Ok(1));
  assert_eq!(queue.capacity().await, Ok(4));
  assert_eq!(queue.poll().await.unwrap(), 10);
}

#[tokio::test(flavor = "multi_thread")]
async fn offer_waits_until_slot_available() {
  let queue: TokioMpscQueue<u8> = make_tokio_mpsc_queue(1);
  queue.offer(1).await.unwrap();

  let queue_clone = queue.clone();
  let pending_offer = queue_clone.offer(2);
  pin!(pending_offer);

  tokio::select! {
    _ = &mut pending_offer => panic!("offer should be waiting for capacity"),
    _ = sleep(Duration::from_millis(50)) => {}
  }

  // Consuming the first value should unblock the pending offer.
  assert_eq!(queue.poll().await.unwrap(), 1);
  pending_offer.await.expect("offer should eventually succeed");
  assert_eq!(queue.poll().await.unwrap(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn close_prevents_further_operations() {
  let queue: TokioMpscQueue<&'static str> = make_tokio_mpsc_queue(2);

  queue.offer("hello").await.unwrap();
  queue.close().await.unwrap();
  assert_eq!(queue.poll().await.unwrap(), "hello");
  assert_eq!(queue.poll().await.err(), Some(QueueError::Disconnected));
  assert!(matches!(queue.offer("world").await.err(), Some(QueueError::Closed(msg)) if msg == "world"));
}
