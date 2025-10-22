//! Scheduler Latency Benchmark
//!
//! ReadyQueueCoordinator操作のレイテンシを測定します。
//!
//! 測定項目:
//! - register_ready操作のレイテンシ
//! - drain_ready_cycle操作のレイテンシ
//! - handle_invoke_result操作のレイテンシ
//! - エンドツーエンド処理時間（register → drain → handle）

use std::time::Duration;

use cellex_actor_core_rs::api::actor_scheduler::{
  DefaultReadyQueueCoordinator, InvokeResult, MailboxIndex, ReadyQueueCoordinator,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration};

/// register_ready操作のレイテンシ測定（単一操作）
fn bench_register_ready_latency(c: &mut Criterion) {
  let mut group = c.benchmark_group("register_ready_latency");
  group.measurement_time(Duration::from_secs(5));
  group.plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

  // Empty queue (best case)
  group.bench_function("empty_queue", |b| {
    let mut coordinator = DefaultReadyQueueCoordinator::new(32);
    let mut counter = 0u32;

    b.iter(|| {
      coordinator.register_ready(MailboxIndex::new(counter, 0));
      counter += 1;
    });
  });

  // After 100 items already queued
  group.bench_function("queue_size_100", |b| {
    let mut coordinator = DefaultReadyQueueCoordinator::new(32);

    // Pre-fill queue
    for i in 0..100 {
      coordinator.register_ready(MailboxIndex::new(i, 0));
    }

    let mut counter = 100u32;
    b.iter(|| {
      coordinator.register_ready(MailboxIndex::new(counter, 0));
      counter += 1;
    });
  });

  // After 1000 items already queued
  group.bench_function("queue_size_1000", |b| {
    let mut coordinator = DefaultReadyQueueCoordinator::new(32);

    // Pre-fill queue
    for i in 0..1000 {
      coordinator.register_ready(MailboxIndex::new(i, 0));
    }

    let mut counter = 1000u32;
    b.iter(|| {
      coordinator.register_ready(MailboxIndex::new(counter, 0));
      counter += 1;
    });
  });

  // Duplicate registration (worst case)
  group.bench_function("duplicate_registration", |b| {
    let mut coordinator = DefaultReadyQueueCoordinator::new(32);
    let idx = MailboxIndex::new(0, 0);

    // Register once
    coordinator.register_ready(idx);

    b.iter(|| {
      coordinator.register_ready(idx);
    });
  });

  group.finish();
}

/// drain_ready_cycle操作のレイテンシ測定
fn bench_drain_latency(c: &mut Criterion) {
  let mut group = c.benchmark_group("drain_latency");
  group.measurement_time(Duration::from_secs(5));

  for queue_size in [10, 100, 1_000, 10_000].iter() {
    for batch_size in [10, 32, 100].iter() {
      group.bench_with_input(
        BenchmarkId::new(format!("queue_{}", queue_size), batch_size),
        &(*queue_size, *batch_size),
        |b, &(queue_size, batch_size)| {
          b.iter_batched(
            || {
              let mut coordinator = DefaultReadyQueueCoordinator::new(32);
              for i in 0..queue_size {
                coordinator.register_ready(MailboxIndex::new(i as u32, 0));
              }
              (coordinator, Vec::with_capacity(batch_size))
            },
            |(mut coordinator, mut out)| {
              coordinator.drain_ready_cycle(batch_size, &mut out);
              black_box(out.len());
            },
            criterion::BatchSize::SmallInput,
          );
        },
      );
    }
  }

  group.finish();
}

/// handle_invoke_result操作のレイテンシ測定
fn bench_handle_invoke_result_latency(c: &mut Criterion) {
  let mut group = c.benchmark_group("handle_invoke_result_latency");

  let results = vec![
    ("completed_ready", InvokeResult::Completed { ready_hint: true }),
    ("completed_not_ready", InvokeResult::Completed { ready_hint: false }),
    ("yielded", InvokeResult::Yielded),
    ("stopped", InvokeResult::Stopped),
  ];

  for (name, result) in results {
    group.bench_function(name, |b| {
      let mut coordinator = DefaultReadyQueueCoordinator::new(32);
      let idx = MailboxIndex::new(0, 0);

      b.iter(|| {
        coordinator.handle_invoke_result(idx, result.clone());
      });
    });
  }

  group.finish();
}

/// unregister操作のレイテンシ測定
fn bench_unregister_latency(c: &mut Criterion) {
  let mut group = c.benchmark_group("unregister_latency");

  for queue_size in [10, 100, 1_000].iter() {
    group.bench_with_input(BenchmarkId::from_parameter(queue_size), queue_size, |b, &queue_size| {
      b.iter_batched(
        || {
          let mut coordinator = DefaultReadyQueueCoordinator::new(32);
          for i in 0..queue_size {
            coordinator.register_ready(MailboxIndex::new(i as u32, 0));
          }
          coordinator
        },
        |mut coordinator| {
          coordinator.unregister(MailboxIndex::new(queue_size as u32 / 2, 0));
        },
        criterion::BatchSize::SmallInput,
      );
    });
  }

  group.finish();
}

/// エンドツーエンド処理時間測定（register → drain → handle）
fn bench_end_to_end_latency(c: &mut Criterion) {
  let mut group = c.benchmark_group("end_to_end_latency");
  group.measurement_time(Duration::from_secs(10));

  for num_items in [1, 10, 100].iter() {
    group.bench_with_input(BenchmarkId::new("items", num_items), num_items, |b, &num_items| {
      b.iter(|| {
        let mut coordinator = DefaultReadyQueueCoordinator::new(32);
        let mut out = Vec::with_capacity(num_items);

        // 1. Register
        for i in 0..num_items {
          coordinator.register_ready(MailboxIndex::new(i as u32, 0));
        }

        // 2. Drain
        coordinator.drain_ready_cycle(num_items, &mut out);

        // 3. Handle results
        for idx in &out {
          coordinator.handle_invoke_result(*idx, InvokeResult::Completed { ready_hint: false });
        }

        black_box(out.len());
      });
    });
  }

  group.finish();
}

/// パーセンタイル測定（p50, p95, p99, p99.9）
fn bench_latency_percentiles(c: &mut Criterion) {
  let mut group = c.benchmark_group("latency_percentiles");
  group.measurement_time(Duration::from_secs(10));
  group.sample_size(1000); // More samples for accurate percentiles

  group.bench_function("register_drain_cycle", |b| {
    b.iter(|| {
      let mut coordinator = DefaultReadyQueueCoordinator::new(32);
      let num_items = 100;
      let mut out = Vec::with_capacity(num_items);

      // Register
      for i in 0..num_items {
        coordinator.register_ready(MailboxIndex::new(i as u32, 0));
      }

      // Drain
      coordinator.drain_ready_cycle(num_items, &mut out);

      black_box(out.len());
    });
  });

  group.finish();
}

/// ワーストケースレイテンシ測定
fn bench_worst_case_latency(c: &mut Criterion) {
  let mut group = c.benchmark_group("worst_case_latency");

  // Scenario 1: Many duplicates
  group.bench_function("many_duplicates", |b| {
    b.iter(|| {
      let mut coordinator = DefaultReadyQueueCoordinator::new(32);
      let idx = MailboxIndex::new(0, 0);

      // Register same index 1000 times
      for _ in 0..1000 {
        coordinator.register_ready(idx);
      }

      let mut out = Vec::with_capacity(1);
      coordinator.drain_ready_cycle(1000, &mut out);

      black_box(out.len()); // Should be 1
    });
  });

  // Scenario 2: Interleaved register/drain
  group.bench_function("interleaved_register_drain", |b| {
    b.iter(|| {
      let mut coordinator = DefaultReadyQueueCoordinator::new(32);
      let mut out = Vec::with_capacity(10);

      for i in 0..100 {
        coordinator.register_ready(MailboxIndex::new(i, 0));

        if i % 10 == 9 {
          coordinator.drain_ready_cycle(10, &mut out);
          out.clear();
        }
      }

      black_box(out.len());
    });
  });

  group.finish();
}

criterion_group!(
  benches,
  bench_register_ready_latency,
  bench_drain_latency,
  bench_handle_invoke_result_latency,
  bench_unregister_latency,
  bench_end_to_end_latency,
  bench_latency_percentiles,
  bench_worst_case_latency,
);
criterion_main!(benches);
