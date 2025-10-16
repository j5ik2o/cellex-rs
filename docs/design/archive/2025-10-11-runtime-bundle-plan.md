# Actor Runtime バンドル計画 (2025-10-11)

## 現状サマリ (2025-10-13)
- `GenericActorRuntime` が scheduler / mailbox / receive-timeout / metrics / event ハンドラを束ねるハブとして機能し、Tokio・Embassy 拡張トレイトから差し替え可能。
- `PriorityMailboxSpawnerHandle` により scheduler 側は mailbox 実装へ非依存でアクター生成を行えている。
- ReceiveTimeoutDriver・EventListener/EscalationHandler の統合は GenericActorRuntime 側で完了し、Tokio ルートでは実際に運用中。

## 未解決課題
- [MUST] フェーズ 3（ReceiveTimeout / Event / Metrics の最終統合、RuntimeBundle Builder）の実装を完了し、API とテストを揃える。
- [SHOULD] `GenericActorRuntime::host()` / `::embedded()` / `::remote()` といったプリセットを提供し、各プロファイルでのクロスビルド（`thumbv6m-none-eabi`, `thumbv8m.main-none-eabi` 含む）を CI に追加する。
- [SHOULD] `ActorSystemBuilder` を導入し、 `ActorSystem::new` からの移行レイヤーと Config/Metrics/Telemetry 等のチェーン API を提供する。
- [MUST] `PrometheusMetricsSink` / `DefmtMetricsSink` などプラットフォーム別メトリクス実装を整備し、GenericActorRuntime へ組み込む。
- [MUST] `EmbassyReceiveTimeoutDriver` を仕上げて Embedded プロファイルへ組み込み、割り込み駆動タイマのテストを追加する。
- [MUST] EventListener / EscalationHandler の Embedded・Remote デフォルトと FailureHub 連携テストを整備する。
- [SHOULD] MailboxBuilder / MailboxHandle の Embedded（heapless）・Remote 向け実装および `RuntimeBundleBuilder` の setter を追加する。
- [SHOULD] `PriorityMailboxSpawnerHandle` の命名と責務を見直し、メトリクス注入経路のテストカバレッジを拡充する。
- [SHOULD] README / ワークノート等のドキュメントを更新し、新しい実行モデルと Builder の利用手順を明記する。

## 優先アクション
1. RuntimeBundle Builder と `GenericActorRuntime::{host,embedded,remote}` の最小実装を追加し、クロスビルド＆統合テスト（Tokio/Embassy）を実行する。
2. `ActorSystemBuilder` のドラフトを実装し、旧 API からの互換層と Config/Metrics のマージ順序を固定する。
3. MetricsSink と ReceiveTimeoutDriver（Prometheus / Defmt / Embassy）の初期実装と統合テストを用意し、FailureHub/イベント連携の検証を進める。
4. ドキュメント更新・サンプル整備を行い、Runtime バンドルのプリセット選択と Builder フローを開発者向けに周知する。
