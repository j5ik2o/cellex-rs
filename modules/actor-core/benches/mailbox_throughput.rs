//! Mailbox Throughput Benchmark
//!
//! アクターのメッセージスループットを測定します。
//!
//! 測定シナリオ:
//! - 1 actor × 100k messages
//! - 10 actors × 10k messages
//! - 100 actors × 1k messages
//! - 1000 actors × 100 messages

use cellex_actor_core_rs::api::actor_scheduler::{
  DefaultReadyQueueCoordinator, InvokeResult, MailboxIndex, ReadyQueueCoordinator,
};
#[cfg(feature = "new-scheduler")]
use cellex_actor_core_rs::api::actor_scheduler::{LockFreeCoordinator, LockFreeCoordinatorV2, ReadyQueueCoordinatorV2};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// 基本的なregister/drain操作のスループット測定
fn bench_register_drain_throughput(c: &mut Criterion) {
  let mut group = c.benchmark_group("coordinator_throughput");

  for size in [100, 1_000, 10_000, 100_000].iter() {
    group.throughput(Throughput::Elements(*size as u64));
    group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
      b.iter(|| {
        let mut coordinator = DefaultReadyQueueCoordinator::new(32);
        let mut out = Vec::with_capacity(size);

        // Register phase
        for i in 0..size {
          coordinator.register_ready(MailboxIndex::new(i as u32, 0));
        }

        // Drain phase
        coordinator.drain_ready_cycle(size, &mut out);

        black_box(out.len());
      });
    });
  }

  group.finish();
}

/// 重複登録の検出オーバーヘッド測定
fn bench_duplicate_detection(c: &mut Criterion) {
  let mut group = c.benchmark_group("duplicate_detection");

  for dup_ratio in [0, 25, 50, 75].iter() {
    group.bench_with_input(
      BenchmarkId::new("duplicate_ratio", dup_ratio),
      dup_ratio,
      |b, &dup_ratio| {
        b.iter(|| {
          let mut coordinator = DefaultReadyQueueCoordinator::new(32);
          let total = 10_000;
          let unique = total * (100 - dup_ratio) / 100;

          // Register with duplicates
          for i in 0..total {
            let idx = i % unique;
            coordinator.register_ready(MailboxIndex::new(idx as u32, 0));
          }

          let mut out = Vec::with_capacity(unique);
          coordinator.drain_ready_cycle(unique, &mut out);

          // Should only contain unique entries
          black_box(out.len());
        });
      },
    );
  }

  group.finish();
}

/// バッチサイズによる影響測定
fn bench_batch_size_impact(c: &mut Criterion) {
  let mut group = c.benchmark_group("batch_size_impact");
  let total_items = 10_000;

  for batch_size in [1, 10, 32, 64, 128, 256, 512].iter() {
    group.bench_with_input(
      BenchmarkId::from_parameter(batch_size),
      batch_size,
      |b, &batch_size| {
        b.iter(|| {
          let mut coordinator = DefaultReadyQueueCoordinator::new(32);
          let mut out = Vec::with_capacity(batch_size);

          // Register all items
          for i in 0..total_items {
            coordinator.register_ready(MailboxIndex::new(i as u32, 0));
          }

          // Drain in batches
          let mut drained = 0;
          while drained < total_items {
            coordinator.drain_ready_cycle(batch_size, &mut out);
            drained += out.len();
            out.clear();
          }

          black_box(drained);
        });
      },
    );
  }

  group.finish();
}

/// 並行register操作のスケーラビリティ測定
fn bench_concurrent_register(c: &mut Criterion) {
  let mut group = c.benchmark_group("concurrent_register");
  group.measurement_time(Duration::from_secs(10));

  for num_threads in [1, 2, 4, 8].iter() {
    group.bench_with_input(
      BenchmarkId::new("threads", num_threads),
      num_threads,
      |b, &num_threads| {
        b.iter(|| {
          let coordinator = Arc::new(Mutex::new(DefaultReadyQueueCoordinator::new(32)));
          let items_per_thread = 10_000usize;

          let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
              let coordinator_clone = Arc::clone(&coordinator);
              thread::spawn(move || {
                for i in 0..items_per_thread {
                  let idx = (thread_id * items_per_thread + i) as u32;
                  coordinator_clone
                    .lock()
                    .unwrap()
                    .register_ready(MailboxIndex::new(idx, 0));
                }
              })
            })
            .collect();

          for handle in handles {
            handle.join().unwrap();
          }

          let total_expected = num_threads * items_per_thread;
          let mut out = Vec::with_capacity(total_expected);
          coordinator
            .lock()
            .unwrap()
            .drain_ready_cycle(total_expected, &mut out);

          black_box(out.len());
        });
      },
    );
  }

  group.finish();
}

/// handle_invoke_result操作のオーバーヘッド測定
fn bench_invoke_result_handling(c: &mut Criterion) {
  let mut group = c.benchmark_group("invoke_result_handling");

  let scenarios = vec![
    ("completed_ready", InvokeResult::Completed { ready_hint: true }),
    (
      "completed_not_ready",
      InvokeResult::Completed { ready_hint: false },
    ),
    ("yielded", InvokeResult::Yielded),
    ("stopped", InvokeResult::Stopped),
  ];

  for (name, result) in scenarios {
    group.bench_function(name, |b| {
      b.iter(|| {
        let mut coordinator = DefaultReadyQueueCoordinator::new(32);
        let num_items = 1_000usize;

        // Register items
        for i in 0..num_items {
          coordinator.register_ready(MailboxIndex::new(i as u32, 0));
        }

        // Drain and handle results
        let mut out = Vec::with_capacity(num_items);
        coordinator.drain_ready_cycle(num_items, &mut out);

        for idx in &out {
          coordinator.handle_invoke_result(*idx, result.clone());
        }

        black_box(out.len());
      });
    });
  }

  group.finish();
}

/// unregister操作のパフォーマンス測定
fn bench_unregister_performance(c: &mut Criterion) {
  let mut group = c.benchmark_group("unregister_performance");

  for size in [100, 1_000, 10_000].iter() {
    group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
      b.iter(|| {
        let mut coordinator = DefaultReadyQueueCoordinator::new(32);

        // Register items
        for i in 0..size {
          coordinator.register_ready(MailboxIndex::new(i as u32, 0));
        }

        // Unregister half
        for i in 0..(size / 2) {
          coordinator.unregister(MailboxIndex::new(i as u32, 0));
        }

        // Drain remaining
        let mut out = Vec::with_capacity(size / 2);
        coordinator.drain_ready_cycle(size, &mut out);

        black_box(out.len());
      });
    });
  }

  group.finish();
}

/// 両実装の並行性能比較ベンチマーク (new-scheduler feature required)
#[cfg(feature = "new-scheduler")]
fn bench_concurrent_comparison(c: &mut Criterion) {
  let mut group = c.benchmark_group("concurrent_comparison");
  group.measurement_time(Duration::from_secs(10));

  for num_threads in [1, 2, 4, 8].iter() {
    // DefaultReadyQueueCoordinator (Mutex-based)
    group.bench_with_input(
      BenchmarkId::new("default_locked", num_threads),
      num_threads,
      |b, &num_threads| {
        b.iter(|| {
          let coordinator = Arc::new(Mutex::new(DefaultReadyQueueCoordinator::new(32)));
          let items_per_thread = 10_000usize;

          let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
              let coordinator_clone = Arc::clone(&coordinator);
              thread::spawn(move || {
                for i in 0..items_per_thread {
                  let idx = (thread_id * items_per_thread + i) as u32;
                  coordinator_clone
                    .lock()
                    .unwrap()
                    .register_ready(MailboxIndex::new(idx, 0));
                }
              })
            })
            .collect();

          for handle in handles {
            handle.join().unwrap();
          }

          let total_expected = num_threads * items_per_thread;
          let mut out = Vec::with_capacity(total_expected);
          coordinator
            .lock()
            .unwrap()
            .drain_ready_cycle(total_expected, &mut out);

          black_box(out.len());
        });
      },
    );

    // LockFreeCoordinator (DashSet/SegQueue-based)
    group.bench_with_input(
      BenchmarkId::new("lockfree", num_threads),
      num_threads,
      |b, &num_threads| {
        b.iter(|| {
          let coordinator = Arc::new(Mutex::new(LockFreeCoordinator::new(32)));
          let items_per_thread = 10_000usize;

          let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
              let coordinator_clone = Arc::clone(&coordinator);
              thread::spawn(move || {
                for i in 0..items_per_thread {
                  let idx = (thread_id * items_per_thread + i) as u32;
                  coordinator_clone
                    .lock()
                    .unwrap()
                    .register_ready(MailboxIndex::new(idx, 0));
                }
              })
            })
            .collect();

          for handle in handles {
            handle.join().unwrap();
          }

          let total_expected = num_threads * items_per_thread;
          let mut out = Vec::with_capacity(total_expected);
          coordinator
            .lock()
            .unwrap()
            .drain_ready_cycle(total_expected, &mut out);

          black_box(out.len());
        });
      },
    );
  }

  group.finish();
}

/// V1 vs V2 comparison benchmark (new-scheduler feature required)
#[cfg(feature = "new-scheduler")]
fn bench_v1_vs_v2_comparison(c: &mut Criterion) {
  let mut group = c.benchmark_group("v1_vs_v2_comparison");
  group.measurement_time(Duration::from_secs(10));

  for num_threads in [1, 2, 4, 8].iter() {
    // V1: With Mutex wrapper (current)
    group.bench_with_input(
      BenchmarkId::new("v1_with_mutex", num_threads),
      num_threads,
      |b, &num_threads| {
        b.iter(|| {
          let coordinator = Arc::new(Mutex::new(LockFreeCoordinator::new(32)));
          let items_per_thread = 10_000usize;

          let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
              let coordinator_clone = Arc::clone(&coordinator);
              thread::spawn(move || {
                for i in 0..items_per_thread {
                  let idx = (thread_id * items_per_thread + i) as u32;
                  coordinator_clone
                    .lock()
                    .unwrap()
                    .register_ready(MailboxIndex::new(idx, 0));
                }
              })
            })
            .collect();

          for handle in handles {
            handle.join().unwrap();
          }

          let total_expected = num_threads * items_per_thread;
          let mut out = Vec::with_capacity(total_expected);
          coordinator
            .lock()
            .unwrap()
            .drain_ready_cycle(total_expected, &mut out);

          black_box(out.len());
        });
      },
    );

    // V2: Without Mutex wrapper (new)
    group.bench_with_input(
      BenchmarkId::new("v2_no_mutex", num_threads),
      num_threads,
      |b, &num_threads| {
        b.iter(|| {
          let coordinator = Arc::new(LockFreeCoordinatorV2::new(32));
          let items_per_thread = 10_000usize;

          let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
              let coordinator_clone = Arc::clone(&coordinator);
              thread::spawn(move || {
                for i in 0..items_per_thread {
                  let idx = (thread_id * items_per_thread + i) as u32;
                  // No lock needed! ← Key difference
                  coordinator_clone.register_ready(MailboxIndex::new(idx, 0));
                }
              })
            })
            .collect();

          for handle in handles {
            handle.join().unwrap();
          }

          let total_expected = num_threads * items_per_thread;
          let mut out = Vec::with_capacity(total_expected);
          // No lock needed! ← Key difference
          coordinator.drain_ready_cycle(total_expected, &mut out);

          black_box(out.len());
        });
      },
    );
  }

  group.finish();
}

criterion_group!(
  benches,
  bench_register_drain_throughput,
  bench_duplicate_detection,
  bench_batch_size_impact,
  bench_concurrent_register,
  bench_invoke_result_handling,
  bench_unregister_performance,
  bench_concurrent_comparison,
  bench_v1_vs_v2_comparison,
);

criterion_main!(benches);
