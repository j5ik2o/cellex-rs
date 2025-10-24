use std::cell::RefCell;

use cellex_utils_core_rs::{
  collections::queue::ring::ArcSharedRingQueue, sync::RcShared, QueueRw, RingBuffer, RingQueue, RingStorageBackend,
};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

fn make_rc_ring_queue(
  capacity: usize,
) -> RingQueue<RcShared<RingStorageBackend<RcShared<RefCell<RingBuffer<u32>>>>>, u32> {
  let storage = RcShared::new(RefCell::new(RingBuffer::new(capacity)));
  let backend = RcShared::new(RingStorageBackend::new(storage));
  RingQueue::new(backend).with_dynamic(true)
}

fn make_arc_shared_ring_queue(capacity: usize) -> ArcSharedRingQueue<u32> {
  ArcSharedRingQueue::new(capacity).with_dynamic(true)
}

type RcRingQueue = RingQueue<RcShared<RingStorageBackend<RcShared<RefCell<RingBuffer<u32>>>>>, u32>;

fn bench_ring_queue_offer_poll(c: &mut Criterion) {
  let mut group = c.benchmark_group("ring_queue_offer_poll");
  let batch = 128_u32;

  group.bench_function("rc_refcell", |b| {
    b.iter_batched(
      || make_rc_ring_queue(batch as usize),
      |queue: RcRingQueue| {
        for value in 0..batch {
          queue.offer(value).unwrap();
        }
        for _ in 0..batch {
          let _ = queue.poll().unwrap();
        }
      },
      BatchSize::SmallInput,
    );
  });

  group.bench_function("arc_shared_spin", |b| {
    b.iter_batched(
      || make_arc_shared_ring_queue(batch as usize),
      |queue: ArcSharedRingQueue<u32>| {
        for value in 0..batch {
          queue.offer(value).unwrap();
        }
        for _ in 0..batch {
          let _ = queue.poll().unwrap();
        }
      },
      BatchSize::SmallInput,
    );
  });

  group.finish();
}

criterion_group!(benches, bench_ring_queue_offer_poll);
criterion_main!(benches);
