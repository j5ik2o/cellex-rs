#![allow(clippy::disallowed_types)]
use super::*;
use crate::tests::init_arc_critical_section;

fn prepare() {
  init_arc_critical_section();
}

#[test]
fn arc_ring_queue_offer_poll() {
  prepare();
  let queue = ArcLocalRingQueue::new(1);
  queue.offer(1).unwrap();
  assert_eq!(queue.poll().unwrap(), Some(1));
}

#[test]
fn arc_ring_queue_shared_clone() {
  prepare();
  let queue = ArcLocalRingQueue::new(2);
  let cloned = queue.clone();

  queue.offer(5).unwrap();
  cloned.offer(6).unwrap();

  assert_eq!(queue.len().to_usize(), 2);
  assert_eq!(queue.poll().unwrap(), Some(5));
  assert_eq!(cloned.poll().unwrap(), Some(6));
}

#[test]
fn arc_ring_queue_dynamic_and_clean_up() {
  prepare();
  let queue = ArcLocalRingQueue::new(1).with_dynamic(false);
  queue.offer(1).unwrap();
  assert!(matches!(queue.offer(2), Err(QueueError::Full(2))));

  queue.clean_up();
  assert_eq!(queue.len().to_usize(), 0);
}

#[test]
fn arc_ring_queue_capacity_reporting() {
  prepare();
  let queue: ArcRingQueue<u32> = ArcLocalRingQueue::new(2);
  assert!(queue.capacity().is_limitless());

  queue.set_dynamic(false);
  assert_eq!(queue.capacity(), QueueSize::limited(2));
}

#[test]
fn arc_ring_queue_trait_interface() {
  prepare();
  let queue: ArcRingQueue<u32> = ArcLocalRingQueue::new(1).with_dynamic(false);
  queue.offer(4).unwrap();
  assert_eq!(queue.poll().unwrap(), Some(4));
}
