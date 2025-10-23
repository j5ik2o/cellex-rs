use std::thread;

use cellex_utils_core_rs::v2::collections::queue::{OfferOutcome, OverflowPolicy, QueueError};

use super::*;

#[test]
fn fifo_queue_drops_oldest() {
  let queue = make_std_fifo_queue_drop_oldest::<u32>(2);
  assert_eq!(queue.offer(1).unwrap(), OfferOutcome::Enqueued);
  assert_eq!(queue.offer(2).unwrap(), OfferOutcome::Enqueued);
  assert_eq!(queue.offer(3).unwrap(), OfferOutcome::DroppedOldest { count: 1 });
  assert_eq!(queue.poll().unwrap(), 2);
  assert_eq!(queue.poll().unwrap(), 3);
}

#[test]
fn fifo_queue_full_blocking() {
  let queue = make_std_fifo_queue::<u32>(1, OverflowPolicy::Block);
  assert_eq!(queue.offer(10).unwrap(), OfferOutcome::Enqueued);
  let err = queue.offer(20).unwrap_err();
  assert_eq!(err, QueueError::Full);
}

#[test]
fn mpsc_queue_supports_multiple_producers() {
  let queue = make_std_mpsc_queue::<u32>(64, OverflowPolicy::Block);
  let (producer, consumer) = queue.into_mpsc_pair();
  let producer_count = 4;
  let items_per = 8;

  thread::scope(|scope| {
    for base in 0..producer_count {
      let q = producer.clone();
      scope.spawn(move || {
        for offset in 0..items_per {
          let value = base * items_per + offset;
          q.offer(value as u32).unwrap();
        }
      });
    }
  });

  let mut collected = Vec::new();
  for _ in 0..producer_count * items_per {
    collected.push(consumer.poll().unwrap());
  }
  collected.sort();
  let expected: Vec<u32> = (0..(producer_count * items_per) as u32).collect();
  assert_eq!(collected, expected);
}

#[test]
fn spsc_queue_blocks_when_full() {
  let queue = make_std_spsc_queue_blocking::<u32>(1);
  let (producer, consumer) = queue.into_spsc_pair();

  assert_eq!(producer.offer(1).unwrap(), OfferOutcome::Enqueued);
  assert_eq!(consumer.poll().unwrap(), 1);
  assert_eq!(producer.offer(2).unwrap(), OfferOutcome::Enqueued);
  assert_eq!(consumer.poll().unwrap(), 2);
  assert_eq!(producer.offer(3).unwrap(), OfferOutcome::Enqueued);
  assert_eq!(consumer.poll().unwrap(), 3);
}
