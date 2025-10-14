use super::*;
use crate::tests::init_arc_critical_section;
use cellex_utils_core_rs::{QueueBase, QueueRw};

fn prepare() {
  init_arc_critical_section();
}

#[test]
fn arc_unbounded_offer_poll() {
  prepare();
  let queue: ArcMpscUnboundedQueue<u32> = ArcMpscUnboundedQueue::new();
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();
  assert_eq!(queue.len().to_usize(), 2);
  assert_eq!(queue.poll().unwrap(), Some(1));
  assert_eq!(queue.poll().unwrap(), Some(2));
  assert_eq!(queue.poll().unwrap(), None);
}

#[test]
fn arc_unbounded_clean_up_signals_disconnect() {
  prepare();
  let queue: ArcMpscUnboundedQueue<u8> = ArcMpscUnboundedQueue::new();
  queue.offer(9).unwrap();
  queue.clean_up();

  assert!(matches!(queue.poll(), Err(QueueError::Disconnected)));
  assert!(matches!(queue.offer(1), Err(QueueError::Closed(1))));
}

#[test]
fn arc_unbounded_offer_poll_via_traits() {
  prepare();
  let queue: ArcMpscUnboundedQueue<u32> = ArcMpscUnboundedQueue::new();
  queue.offer(7).unwrap();
  assert_eq!(queue.poll().unwrap(), Some(7));
}

#[test]
fn arc_unbounded_capacity_reports_limitless() {
  prepare();
  let queue: ArcMpscUnboundedQueue<u32> = ArcMpscUnboundedQueue::new();
  assert!(queue.capacity().is_limitless());
}
