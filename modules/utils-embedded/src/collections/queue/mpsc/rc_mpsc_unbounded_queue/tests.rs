use super::*;
use cellex_utils_core_rs::{QueueBase, QueueRw};

#[test]
fn rc_unbounded_offer_poll() {
  let queue: RcMpscUnboundedQueue<u32> = RcMpscUnboundedQueue::new();
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();
  assert_eq!(queue.len().to_usize(), 2);
  assert_eq!(queue.poll().unwrap(), Some(1));
  assert_eq!(queue.poll().unwrap(), Some(2));
  assert_eq!(queue.poll().unwrap(), None);
}

#[test]
fn rc_unbounded_clean_up_signals_disconnected() {
  let queue: RcMpscUnboundedQueue<u8> = RcMpscUnboundedQueue::new();
  queue.offer(1).unwrap();
  queue.clean_up();

  assert!(matches!(queue.poll(), Err(QueueError::Disconnected)));
  assert!(matches!(queue.offer(2), Err(QueueError::Closed(2))));
}

#[test]
fn rc_unbounded_offer_poll_via_traits() {
  let mut queue: RcMpscUnboundedQueue<u32> = RcMpscUnboundedQueue::new();
  queue.offer_mut(1).unwrap();
  assert_eq!(queue.poll_mut().unwrap(), Some(1));
}
