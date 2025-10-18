use cellex_utils_core_rs::{QueueBase, QueueReader, QueueWriter};

use super::*;

#[derive(Debug, Clone)]
struct Msg(i32, i8);

impl PriorityMessage for Msg {
  fn get_priority(&self) -> Option<i8> {
    Some(self.1)
  }
}

#[test]
fn rc_priority_queue_orders() {
  let queue: RcPriorityQueue<Msg> = RcPriorityQueue::new(4);
  queue.offer(Msg(1, 0)).unwrap();
  queue.offer(Msg(5, 7)).unwrap();
  queue.offer(Msg(3, 3)).unwrap();

  assert_eq!(queue.poll().unwrap().unwrap().0, 5);
  assert_eq!(queue.poll().unwrap().unwrap().0, 3);
  assert_eq!(queue.poll().unwrap().unwrap().0, 1);
  assert!(queue.poll().unwrap().is_none());
}

#[test]
fn rc_priority_queue_len_capacity_updates() {
  let queue: RcPriorityQueue<Msg> = RcPriorityQueue::new(2);
  assert_eq!(queue.len(), QueueSize::limited(0));

  queue.offer(Msg(1, 0)).unwrap();
  assert_eq!(queue.len(), QueueSize::limited(1));

  queue.clean_up();
  assert_eq!(queue.len(), QueueSize::limited(0));
}

#[test]
fn rc_priority_queue_len_across_levels() {
  let queue: RcPriorityQueue<Msg> = RcPriorityQueue::new(2);
  queue.offer(Msg(1, 0)).unwrap();
  queue.offer(Msg(2, 5)).unwrap();
  assert_eq!(queue.len(), QueueSize::limited(2));
}

#[test]
fn rc_priority_queue_capacity_behaviour() {
  let queue: RcPriorityQueue<Msg> = RcPriorityQueue::new(1);
  assert!(queue.capacity().is_limitless());

  queue.set_dynamic(false);
  let expected = QueueSize::limited(PRIORITY_LEVELS);
  assert_eq!(queue.capacity(), expected);
}

#[test]
fn rc_priority_queue_trait_cleanup() {
  let mut queue: RcPriorityQueue<Msg> = RcPriorityQueue::new(2).with_dynamic(false);
  queue.offer_mut(Msg(1, 0)).unwrap();
  queue.offer_mut(Msg(2, 1)).unwrap();
  assert_eq!(queue.poll_mut().unwrap().unwrap().0, 2);
  queue.clean_up_mut();
  assert!(queue.poll_mut().unwrap().is_none());
}

#[test]
fn rc_priority_queue_priority_clamp_and_default() {
  #[derive(Debug, Clone)]
  struct OptionalPriority(i32, Option<i8>);

  impl PriorityMessage for OptionalPriority {
    fn get_priority(&self) -> Option<i8> {
      self.1
    }
  }

  let queue: RcPriorityQueue<OptionalPriority> = RcPriorityQueue::new(1).with_dynamic(false);
  queue.offer(OptionalPriority(1, Some(127))).unwrap();
  queue.offer(OptionalPriority(2, Some(-128))).unwrap();
  queue.offer(OptionalPriority(3, None)).unwrap();

  assert_eq!(queue.poll().unwrap().unwrap().0, 1);
  assert_eq!(queue.poll().unwrap().unwrap().0, 3);
  assert_eq!(queue.poll().unwrap().unwrap().0, 2);
}
