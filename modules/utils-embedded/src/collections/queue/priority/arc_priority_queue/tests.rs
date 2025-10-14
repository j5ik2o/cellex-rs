use super::*;
use crate::tests::init_arc_critical_section;
use cellex_utils_core_rs::{QueueBase, QueueRw};

fn prepare() {
  init_arc_critical_section();
}

#[derive(Debug, Clone)]
struct Msg(i32, i8);

impl PriorityMessage for Msg {
  fn get_priority(&self) -> Option<i8> {
    Some(self.1)
  }
}

#[test]
fn arc_priority_queue_orders() {
  prepare();
  let queue: ArcLocalPriorityQueue<Msg> = ArcLocalPriorityQueue::new(3);
  queue.offer(Msg(1, 0)).unwrap();
  queue.offer(Msg(9, 7)).unwrap();
  queue.offer(Msg(5, 3)).unwrap();

  assert_eq!(queue.poll().unwrap().unwrap().0, 9);
  assert_eq!(queue.poll().unwrap().unwrap().0, 5);
  assert_eq!(queue.poll().unwrap().unwrap().0, 1);
  assert!(queue.poll().unwrap().is_none());
}

#[test]
fn arc_priority_queue_len_and_clean_up() {
  prepare();
  let queue: ArcLocalPriorityQueue<Msg> = ArcLocalPriorityQueue::new(1);
  queue.offer(Msg(1, 0)).unwrap();
  assert_eq!(queue.len(), QueueSize::limited(1));
  queue.clean_up();
  assert_eq!(queue.len(), QueueSize::limited(0));
}

#[test]
fn arc_priority_queue_len_across_levels() {
  prepare();
  let queue: ArcLocalPriorityQueue<Msg> = ArcLocalPriorityQueue::new(2);
  queue.offer(Msg(1, 0)).unwrap();
  queue.offer(Msg(2, 5)).unwrap();
  assert_eq!(queue.len(), QueueSize::limited(2));
}

#[test]
fn arc_priority_queue_capacity_behaviour() {
  prepare();
  let queue: ArcLocalPriorityQueue<Msg> = ArcLocalPriorityQueue::new(2);
  assert!(queue.capacity().is_limitless());

  queue.set_dynamic(false);
  let expected = QueueSize::limited(2 * PRIORITY_LEVELS);
  assert_eq!(queue.capacity(), expected);
}

#[test]
fn arc_priority_queue_trait_cleanup() {
  prepare();
  let queue: ArcLocalPriorityQueue<Msg> = ArcLocalPriorityQueue::new(2).with_dynamic(false);
  queue.offer(Msg(1, 0)).unwrap();
  queue.offer(Msg(2, 1)).unwrap();
  assert_eq!(queue.poll().unwrap().unwrap().0, 2);
  queue.clean_up();
  assert!(queue.poll().unwrap().is_none());
}

#[test]
fn arc_priority_queue_priority_clamp_and_default() {
  prepare();
  #[derive(Debug, Clone)]
  struct OptionalPriority(i32, Option<i8>);

  impl PriorityMessage for OptionalPriority {
    fn get_priority(&self) -> Option<i8> {
      self.1
    }
  }

  let queue: ArcLocalPriorityQueue<OptionalPriority> = ArcLocalPriorityQueue::new(1).with_dynamic(false);
  queue.offer(OptionalPriority(1, Some(127))).unwrap();
  queue.offer(OptionalPriority(2, Some(-128))).unwrap();
  queue.offer(OptionalPriority(3, None)).unwrap();

  assert_eq!(queue.poll().unwrap().unwrap().0, 1);
  assert_eq!(queue.poll().unwrap().unwrap().0, 3);
  assert_eq!(queue.poll().unwrap().unwrap().0, 2);
}
