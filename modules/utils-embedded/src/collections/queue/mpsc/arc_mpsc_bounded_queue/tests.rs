#![allow(clippy::disallowed_types)]
use cellex_utils_core_rs::{QueueBase, QueueRw};

use super::*;
use crate::tests::init_arc_critical_section;

fn prepare() {
  init_arc_critical_section();
}

#[test]
fn arc_bounded_capacity_limit() {
  prepare();
  let queue: ArcMpscBoundedQueue<u32> = ArcMpscBoundedQueue::new(1);
  queue.offer(10).unwrap();
  let err = queue.offer(11).unwrap_err();
  assert!(matches!(err, QueueError::Full(11)));
}

#[test]
fn arc_bounded_clean_up_closes_queue() {
  prepare();
  let queue: ArcMpscBoundedQueue<u32> = ArcMpscBoundedQueue::new(2);
  queue.offer(1).unwrap();
  queue.clean_up();

  assert!(matches!(queue.poll(), Err(QueueError::Disconnected)));
  assert!(matches!(queue.offer(2), Err(QueueError::Closed(2))));
}

#[test]
fn arc_bounded_reports_len_and_capacity() {
  prepare();
  let queue: ArcMpscBoundedQueue<u32> = ArcMpscBoundedQueue::new(3);
  assert_eq!(queue.capacity(), QueueSize::limited(3));

  queue.offer(1).unwrap();
  assert_eq!(queue.len(), QueueSize::limited(1));
}

#[test]
fn arc_bounded_trait_cleanup_marks_closed() {
  prepare();
  let queue: ArcMpscBoundedQueue<u32> = ArcMpscBoundedQueue::new(2);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();

  queue.clean_up();
  assert!(matches!(queue.poll(), Err(QueueError::Disconnected)));
  assert!(matches!(queue.offer(3), Err(QueueError::Closed(3))));
}
