use super::*;

#[test]
fn ring_queue_offer_poll() {
  let queue = ArcRingQueue::new(2).with_dynamic(false);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();
  assert_eq!(queue.offer(3), Err(QueueError::Full(3)));

  assert_eq!(queue.poll().unwrap(), Some(1));
  assert_eq!(queue.poll().unwrap(), Some(2));
  assert_eq!(queue.poll().unwrap(), None);
}

#[test]
fn ring_queue_shared_clone_observes_state() {
  let queue = ArcRingQueue::new(4);
  let cloned = queue.clone();

  queue.offer(10).unwrap();
  queue.offer(11).unwrap();

  assert_eq!(cloned.len().to_usize(), 2);
  assert_eq!(cloned.poll().unwrap(), Some(10));
  assert_eq!(queue.poll().unwrap(), Some(11));
  assert_eq!(queue.poll().unwrap(), None);
}

#[test]
fn ring_queue_clean_up_resets_queue() {
  let queue = ArcRingQueue::new(2);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();

  queue.clean_up();
  assert_eq!(queue.len().to_usize(), 0);
  assert!(queue.poll().unwrap().is_none());
}

#[test]
fn ring_queue_dynamic_resize() {
  let queue = ArcRingQueue::new(1);
  queue.set_dynamic(true);
  queue.offer(1).unwrap();
  queue.offer(2).unwrap();
  assert!(queue.len().to_usize() >= 2);
}

#[test]
fn ring_queue_capacity_and_poll_via_traits() {
  let mut queue = ArcRingQueue::new(1).with_dynamic(false);
  queue.offer_mut(9).unwrap();
  assert_eq!(queue.capacity(), QueueSize::limited(1));
  assert_eq!(queue.poll_mut().unwrap(), Some(9));
}
