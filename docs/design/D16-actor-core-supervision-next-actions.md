# actor-core Panic / Supervision：次アクション

## 現状メモ
- `unwind-supervision` フィーチャで `catch_unwind` ベースの監督経路を opt-in できるようになり、CI (`scripts/ci-check.sh std`) でも検証されている。
- `BehaviorFailure` / `ActorFailure` を通じた Result ベースのエラー伝搬がデフォルトパス。

## 優先タスク
1. `unwind-supervision` 有効時のコードサイズ／ターゲット要件を調査し、利用可能な MCU をドキュメント化する。
2. `no_std` 向けのログ・レポート手段（例: defmt, panic-probe 連携）を検討し、panic handler のガイドラインを整備する。

## 完了済みタスク（2025-10-20）
- Remote 層: `modules/remote-core/src/tests.rs` に `remote_failure_notifier_triggers_telemetry_metrics` を追加し、`FailureTelemetry` と `MetricsSink` の呼び出しを確認。
- Cluster 層: `modules/cluster-core/src/tests.rs` に `cluster_failure_bridge_triggers_telemetry_metrics` を追加し、ローカル／リモート双方で telemetry が発火することを検証。

## 参考
- 旧メモは `docs/design/archive/2025-10-14-actor-core-supervision.md` を参照。
