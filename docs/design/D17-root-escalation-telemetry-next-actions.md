# Root Escalation Telemetry：次アクション

## 現状メモ
- `FailureTelemetry` 抽象と `SerializerRegistryExtension` 経由のデフォルト注入が実装済みで、`GenericActorSystemConfig::with_failure_telemetry_builder` から差し替え可能。
- `FailureTelemetryShared` / `FailureTelemetryObservationConfig` が追加され、基本的なメトリクス連携は行える。

## 優先タスク
1. thumb ターゲット（`thumbv6m-none-eabi` / `thumbv8m.main-none-eabi`）での `no_std` 動作検証を行い、`FailureTelemetryObservationConfig` の挙動を確認する。
2. Telemetry 呼び出し順と副作用をカバーする統合テストを追加する。
3. `event_handler` / `event_listener` と telemetry の責務分担をドキュメントに明記し、利用者向けガイドを更新する。
4. defmt など `no_std` ログ出力との統合手順を正式化する。

## 参考
- 旧メモは `docs/design/archive/2025-10-14-root-escalation-telemetry-plan.md` を参照。
