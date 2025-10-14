use super::*;

#[derive(Debug, Clone)]
struct Msg(i32, i8);

impl PriorityMessage for Msg {
  fn get_priority(&self) -> Option<i8> {
    Some(self.1)
  }
}

#[test]
fn priority_queue_orders_elements() {
  let queue = ArcPriorityQueue::new(4);
  queue.offer(Msg(10, 1)).unwrap();
  queue.offer(Msg(99, 7)).unwrap();
  queue.offer(Msg(20, 3)).unwrap();

  assert_eq!(queue.poll().unwrap().unwrap().0, 99);
  assert_eq!(queue.poll().unwrap().unwrap().0, 20);
  assert_eq!(queue.poll().unwrap().unwrap().0, 10);
  assert!(queue.poll().unwrap().is_none());
}

#[test]
fn priority_queue_len_capacity_and_clean_up() {
  let queue = ArcPriorityQueue::new(2);
  assert_eq!(queue.len(), QueueSize::limited(0));

  queue.offer(Msg(1, 0)).unwrap();
  assert_eq!(queue.len(), QueueSize::limited(1));

  queue.clean_up();
  assert_eq!(queue.len(), QueueSize::limited(0));
}

#[test]
fn priority_queue_capacity_reflects_levels() {
  let queue = ArcPriorityQueue::<Msg>::new(1);
  assert!(queue.capacity().is_limitless());
}

#[test]
fn priority_queue_offer_via_trait() {
  let mut queue = ArcPriorityQueue::new(2);
  queue.offer_mut(Msg(5, 2)).unwrap();
  assert_eq!(queue.poll_mut().unwrap().unwrap().0, 5);
}
