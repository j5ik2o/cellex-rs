use super::*;

#[test]
fn rc_ring_queue_offer_poll() {
  let queue = RcRingQueue::new(1).with_dynamic(false);
  queue.offer(10).unwrap();
  assert_eq!(queue.poll().unwrap(), Some(10));
  assert_eq!(queue.poll().unwrap(), None);
}

#[test]
fn rc_ring_queue_shared_clone() {
  let queue = RcRingQueue::new(4);
  let cloned = queue.clone();

  queue.offer(1).unwrap();
  cloned.offer(2).unwrap();

  assert_eq!(queue.len().to_usize(), 2);
  assert_eq!(queue.poll().unwrap(), Some(1));
  assert_eq!(cloned.poll().unwrap(), Some(2));
}

#[test]
fn rc_ring_queue_clean_up_resets_state() {
  let queue = RcRingQueue::new(2);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();

  queue.clean_up();
  assert_eq!(queue.len().to_usize(), 0);
  assert!(queue.poll().unwrap().is_none());
}

#[test]
fn rc_ring_queue_dynamic_growth() {
  let queue = RcRingQueue::new(1).with_dynamic(true);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();
  assert_eq!(queue.len().to_usize(), 2);
}

#[test]
fn rc_ring_queue_set_dynamic_switches_mode() {
  let queue = RcRingQueue::new(1);
  queue.set_dynamic(false);
  queue.offer(1).unwrap();
  assert!(matches!(queue.offer(2), Err(QueueError::Full(2))));
}

#[test]
fn rc_ring_queue_trait_interface() {
  let mut queue = RcRingQueue::new(1).with_dynamic(false);
  queue.offer_mut(3).unwrap();
  assert_eq!(queue.poll_mut().unwrap(), Some(3));
}
