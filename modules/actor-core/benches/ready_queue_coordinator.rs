//! DefaultReadyQueueCoordinator マイクロベンチマーク
//!
//! spin::Mutex + VecDeque (+ BTreeSet) 構成の基本性能を記録し、後続の
//! `RingQueue` や lock-free バリアントとの比較指標に利用する。

use std::{hint::black_box, time::Duration, vec::Vec};

use cellex_actor_core_rs::api::actor_scheduler::{
  DefaultReadyQueueCoordinator, InvokeResult, MailboxIndex, ReadyQueueCoordinator,
};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, PlotConfiguration, Throughput};

/// register_ready → drain_ready_cycle を 1 サイクルとしてスループットを測定する。
fn bench_register_drain_throughput(c: &mut Criterion) {
  let mut group = c.benchmark_group("ready_queue_register_drain");
  group.measurement_time(Duration::from_secs(3));
  group.sample_size(50);
  group.plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

  for &batch in &[1_usize, 8, 32, 128, 512, 2048] {
    group.throughput(Throughput::Elements(batch as u64));
    group.bench_with_input(BenchmarkId::from_parameter(batch), &batch, |b, &batch| {
      let mut coordinator = DefaultReadyQueueCoordinator::new(batch.max(32));
      let mut counter = 0_u32;

      b.iter(|| {
        for _ in 0..batch {
          let idx = MailboxIndex::new(counter, 0);
          coordinator.register_ready(idx);
          counter = counter.wrapping_add(1);
        }

        let mut drained = Vec::with_capacity(batch);
        coordinator.drain_ready_cycle(batch, &mut drained);
        black_box(drained.len());
      });
    });
  }

  group.finish();
}

/// register_ready の単体レイテンシを測定し、queueサイズによる差分を可視化する。
fn bench_register_latency(c: &mut Criterion) {
  let mut group = c.benchmark_group("ready_queue_register_latency");
  group.measurement_time(Duration::from_secs(3));
  group.sample_size(50);

  for &queued in &[0_u32, 32, 256, 1_024, 4_096] {
    group.bench_with_input(BenchmarkId::from_parameter(queued), &queued, |b, &queued| {
      b.iter_batched(
        || {
          let mut coordinator = DefaultReadyQueueCoordinator::new(queued.max(32) as usize);
          for i in 0..queued {
            coordinator.register_ready(MailboxIndex::new(i, 0));
          }
          (coordinator, queued)
        },
        |(mut coordinator, queued)| {
          let idx = MailboxIndex::new(queued, 0);
          coordinator.register_ready(idx);
        },
        BatchSize::SmallInput,
      );
    });
  }

  // Duplicate registration パス
  group.bench_function("duplicate", |b| {
    let mut coordinator = DefaultReadyQueueCoordinator::new(32);
    let idx = MailboxIndex::new(42, 0);
    coordinator.register_ready(idx);

    b.iter(|| {
      coordinator.register_ready(idx);
    });
  });

  group.finish();
}

/// handle_invoke_result のパスを計測。queue 操作との組み合わせを把握する。
fn bench_handle_invoke_result(c: &mut Criterion) {
  let mut group = c.benchmark_group("ready_queue_handle_invoke_result");
  group.measurement_time(Duration::from_secs(3));
  group.sample_size(50);

  let scenarios = [
    ("completed_ready", InvokeResult::Completed { ready_hint: true }),
    ("completed_idle", InvokeResult::Completed { ready_hint: false }),
    ("yielded", InvokeResult::Yielded),
    ("stopped", InvokeResult::Stopped),
  ];

  for (name, result) in scenarios.iter() {
    group.bench_function(*name, |b| {
      b.iter_batched(
        || {
          let mut coordinator = DefaultReadyQueueCoordinator::new(32);
          let idx = MailboxIndex::new(0, 0);
          coordinator.register_ready(idx);
          (coordinator, idx, result.clone())
        },
        |(mut coordinator, idx, result)| {
          coordinator.handle_invoke_result(idx, result);
        },
        BatchSize::SmallInput,
      );
    });
  }

  group.finish();
}

criterion_group!(benches, bench_register_drain_throughput, bench_register_latency, bench_handle_invoke_result,);
criterion_main!(benches);
