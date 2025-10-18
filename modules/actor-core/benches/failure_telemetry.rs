use std::hint::black_box;

use cellex_actor_core_rs::api::{
  actor::{actor_failure::ActorFailure, ActorId, ActorPath},
  failure_telemetry::FailureTelemetryShared,
  supervision::{
    failure::FailureInfo,
    telemetry::{FailureSnapshot, FailureTelemetry, NoopFailureTelemetry},
  },
};
use criterion::{criterion_group, criterion_main, Criterion};

fn snapshot_fixture() -> FailureSnapshot {
  let failure = ActorFailure::from_message("bench failure");
  let info = FailureInfo::new(ActorId(1), ActorPath::new(), failure);
  FailureSnapshot::from_failure_info(&info)
}

fn bench_failure_telemetry(c: &mut Criterion) {
  let snapshot = snapshot_fixture();
  let shared = FailureTelemetryShared::new(NoopFailureTelemetry);
  let direct = NoopFailureTelemetry;

  c.bench_function("failure_telemetry_shared", |b| {
    b.iter(|| {
      shared.with_ref(|telemetry| telemetry.on_failure(black_box(&snapshot)));
    });
  });

  c.bench_function("failure_telemetry_direct", |b| {
    b.iter(|| {
      direct.on_failure(black_box(&snapshot));
    });
  });
}

criterion_group!(benches, bench_failure_telemetry);
criterion_main!(benches);
