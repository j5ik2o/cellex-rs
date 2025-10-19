#![allow(clippy::disallowed_types)]
use cellex_utils_core_rs::{QueueBase, QueueRw};

use super::*;

#[test]
fn rc_bounded_capacity_limit() {
  let queue: RcMpscBoundedQueue<u32> = RcMpscBoundedQueue::new(1);
  queue.offer(42).unwrap();
  let err = queue.offer(99).unwrap_err();
  assert!(matches!(err, QueueError::Full(99)));
}

#[test]
fn rc_bounded_clean_up_closes_queue() {
  let queue: RcMpscBoundedQueue<u32> = RcMpscBoundedQueue::new(2);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();

  queue.clean_up();
  assert!(matches!(queue.poll(), Err(QueueError::Disconnected)));
  assert!(matches!(queue.offer(3), Err(QueueError::Closed(3))));
}

#[test]
fn rc_bounded_capacity_tracking() {
  let queue: RcMpscBoundedQueue<u32> = RcMpscBoundedQueue::new(2);
  assert_eq!(queue.capacity().to_usize(), 2);
  queue.offer(1).unwrap();
  assert_eq!(queue.len(), QueueSize::limited(1));
}
