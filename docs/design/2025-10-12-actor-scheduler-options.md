# ActorScheduler 拡張プラン (2025-10-12)

## 現状サマリ (2025-10-13)
- `SchedulerBuilder` 上に `priority` / `immediate` が揃い、テスト向けの `ImmediateScheduler` は `cfg(test|test-support)` で利用できる。
- `actor-std` に `TokioScheduler`、`actor-embedded` に `EmbassyScheduler` が追加され、ランタイム別にラッパを差し替える土台が整った。
- `ActorRuntimeBundleTokioExt` / `ActorRuntimeBundleEmbassyExt` から scheduler を差し替える拡張メソッドが提供されている。

## 未解決課題
- [MUST] `ActorSystemBuilder` / RuntimeEnv から scheduler を明示的に選択する API を提供し、アプリケーションがビルダー経由で切り替えられるようにする。
- [MUST] `EmbassyScheduler` の統合テスト（シミュレーションでも可）を追加し、`embedded_rc` / `embedded_arc` 構成での動作を自動検証する。
- [SHOULD] スケジューラ別のメトリクス／負荷試験を整備し、Tokio / Embassy / Immediate の挙動差を可視化する。
- [SHOULD] スケジューラ差し替え手順とベストプラクティスをドキュメント化する。

## 優先アクション
1. `ActorSystemBuilder` に `with_scheduler_builder`（仮）を追加し、RuntimeBundle と一緒に scheduler を構成できるようにする。
2. Embassy 用の統合テストまたはベンチ（`embassy_executor` を用いた smoke テスト）を追加し、CI でクロスビルドを実行する。
3. スケジューラごとのベンチマーク／メトリクス収集を行い、結果を docs/worknotes へ記録する。
