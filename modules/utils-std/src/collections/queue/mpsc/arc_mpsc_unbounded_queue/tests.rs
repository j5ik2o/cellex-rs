#![allow(clippy::disallowed_types)]
use super::*;

#[test]
fn unbounded_queue_offer_poll_cycle() {
  let queue: ArcMpscUnboundedQueue<u32> = ArcMpscUnboundedQueue::new();
  queue.offer(10).unwrap();
  queue.offer(20).unwrap();

  assert_eq!(queue.len().to_usize(), 2);
  assert_eq!(queue.poll().unwrap(), Some(10));
  assert_eq!(queue.poll().unwrap(), Some(20));
  assert_eq!(queue.poll().unwrap(), None);
}

#[test]
fn unbounded_queue_closed_state() {
  let queue: ArcMpscUnboundedQueue<u32> = ArcMpscUnboundedQueue::new();
  queue.offer(1).unwrap();
  queue.clean_up();
  assert!(matches!(queue.poll(), Err(QueueError::Disconnected)));
  assert!(matches!(queue.offer(2), Err(QueueError::Closed(2))));
}

#[test]
fn unbounded_queue_ring_buffer_constructor() {
  let queue = ArcMpscUnboundedQueue::with_ring_buffer();
  queue.offer(1).unwrap();
  assert_eq!(queue.poll().unwrap(), Some(1));
}
