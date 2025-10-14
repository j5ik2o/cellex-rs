use super::*;

#[test]
fn bounded_queue_respects_capacity() {
  let queue = ArcMpscBoundedQueue::new(1);
  queue.offer(1).unwrap();
  let err = queue.offer(2).unwrap_err();
  assert!(matches!(err, QueueError::Full(2)));
}

#[test]
fn bounded_queue_poll_returns_items() {
  let queue = ArcMpscBoundedQueue::new(2);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();

  assert_eq!(queue.len().to_usize(), 2);
  assert_eq!(queue.poll().unwrap(), Some(1));
  assert_eq!(queue.poll().unwrap(), Some(2));
  assert_eq!(queue.poll().unwrap(), None);
}

#[test]
fn bounded_queue_clean_up_closes_channel() {
  let queue = ArcMpscBoundedQueue::new(1);
  queue.offer(1).unwrap();
  queue.clean_up();

  assert!(matches!(queue.poll(), Err(QueueError::Disconnected)));
  assert!(matches!(queue.offer(2), Err(QueueError::Closed(2))));
}

#[test]
fn bounded_queue_ring_buffer_constructor() {
  let queue = ArcMpscBoundedQueue::with_ring_buffer(1);
  queue.offer(1).unwrap();
  assert!(matches!(queue.offer(2), Err(QueueError::Full(2))));
}
