# Telemetry Builder を用いた `defmt` 連携検証メモ（2025-10-14）

## 目的

- `FailureTelemetryBuilderShared` / `FailureTelemetryObservationConfig` を利用して、`defmt` ベースの telemetry 実装が `no_std` プロファイルでも組み込めることを確認する。
- 追加コンポーネントが `alloc` だけで構成できるかを確認し、今後のフェーズ3拡張（組み込み向けログ）に備える。

## 方針

1. `panic-probe` + `defmt` を利用した最小 telemetry を想定。
2. ビルダー API を使い、`GenericActorSystemConfig::with_failure_telemetry_builder` 経由で `defmt` 版を注入する。
3. 観測フック (`FailureTelemetryObservationConfig`) は `MetricsSink` 未設定でも no-op であることを確認。

## サンプルコード

```rust
use cellex_actor_core_rs::{
  FailureTelemetry, FailureTelemetryBuilderShared, FailureTelemetryShared, TelemetryContext,
};

struct DefmtTelemetry;

impl FailureTelemetry for DefmtTelemetry {
  fn on_failure(&self, snapshot: &FailureSnapshot) {
    defmt::error!("telemetry", actor = snapshot.actor().0, message = snapshot.description());
  }
}

fn defmt_builder(_ctx: &TelemetryContext) -> FailureTelemetryShared {
  FailureTelemetryShared::new(DefmtTelemetry)
}

let builder = FailureTelemetryBuilderShared::new(defmt_builder);
let config = GenericActorSystemConfig::default()
  .with_failure_telemetry_builder(Some(builder))
  .with_failure_observation_config(None); // no metrics => no-op 観測
```

- `TelemetryContext` の `metrics_sink` / `extensions` は所有型なので、`no_std` ビルドでも所有権を移すだけで完結。
- 観測設定を指定しない場合は、`FailureTelemetryObservationConfig::new()` が戻るため、`Instant` を使用しない no-op。

## 確認事項

- `scripts/ci-check.sh no-std` で `alloc` 構成（std 無し）でもビルドが成功することを確認済み（2025-10-14）。
- `defmt` への依存はアプリ側（例：`platform-tests/`）で既に利用実績があるため、builder から注入するだけで連携可能。

## TODO

- 実際の `defmt` telemetry 実装は `platform-tests` などの実行パスで追加検証する。
- 観測メトリクス（Latency）を `defmt` 側に流す必要があるかは要検討。
