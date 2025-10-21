# ActorRuntime リファクタ状況 (2025-10-15 更新)

## 現状サマリ
- 旧 `ActorSystemParts` は 2025-10-15 時点でリポジトリから削除済み。`ActorSystem::new` / `::new_with_runtime` は `GenericActorRuntime`（`ActorRuntime` トレイト実装）を受け取り、メールボックス／FailureHub／メトリクス／ReceiveTimeout を束ねる。
- `GenericActorRuntime` は Tokio / Embassy 拡張トレイトから差し替え可能で、`PriorityMailboxSpawnerHandle` を通じてスケジューラを抽象化している。
- `ActorRuntime` トレイトにより GenericActorRuntime の API 群（receive-timeout factory/driver, metrics sink, root event listener, escalation handler）が統合済み。
- `ActorSystemParts` 由来の利用者向け複数戻り値は廃止され、ActorSystem はランタイムを一括で受け取る形に整理されている。
- Tokio / Embassy 向けには `TokioActorRuntime` / `EmbassyActorRuntime` のプリセットを導入し、`tokio_actor_runtime()` / `embassy_actor_runtime()` で簡易生成可能にした。
- `ActorSystem::builder(runtime)` により Runtime 既定値へ `GenericActorSystemConfig` を段階的に適用できるビルダー API を提供した。

## 完了済みトピック
- `RuntimeEnv` から `GenericActorRuntime` への改名と、Tokio / Embassy それぞれのプリセット提供を完了。
- `ActorSystem::builder(runtime)` の導入により、Runtime 既定値 → Config 上書きの順序をコード上で明示した。

## 依然残る課題
- **MUST**: Runtime 層と `GenericActorSystemConfig` の責務を明文化する。Runtime 側はランタイム固有の既定値（メールボックス、ReceiveTimeout、FailureHub、Metrics 等）を管理し、Config 側は利用者がインスタンス毎に上書きする値（ready_queue_worker_count、Spawn ミドルウェア、Extensions など）を扱う。設定優先順位は Runtime → Config の順とし、コード／ドキュメント双方で保証する。
- **MUST**: プラットフォーム別 ReceiveTimeoutDriver・EventListener・MetricsSink（Prometheus/Defmt 等）を整備し、Runtime プリセットに組み込む。
- **MUST**: Embedded/Remote プロファイルで FailureHub 連携と ReceiveTimeout の統合テストを追加し、`cargo check --target thumb-*` を CI に組み込む。
- **SHOULD**: README や設計メモ、サンプルコードを新しい命名／Builder API に合わせて更新し、旧 `ActorRuntimeCore` からの移行手順を案内する。
- **SHOULD**: Runtime プリセットと Spawn ミドルウェアとの連携を整理し、テストカバレッジを拡充する。

## 設計方針（継続）
- `ActorRuntime` トレイトは facade として維持しつつ、共通処理を `GenericActorRuntime` に集約する。Tokio/Embassy/Remote 向けの各 Runtime 実装はこの Core を内包し、プラットフォーム固有の差分を最小化する。
- プラットフォーム差し替えはプリセット Builder を介して行い、詳細調整が必要な場合は拡張トレイト（Tokio/Embassy 等）で提供する。Builder API では「Runtime 既定値 → Config 上書き」の順で適用することを保証する。
- Spawn ミドルウェアや ReadyQueue スケジューラと連携するため、Runtime 経由で命名・メトリクス・イベントハンドラが渡せることを保証する。

## 次アクション
1. Runtime と Config の責務分離をドキュメント化し、優先順位を明文化する。
2. README・設計メモ・サンプルコードを新命名＋Builder API に合わせて更新し、移行ガイドを用意する。
3. プラットフォーム別 ReceiveTimeout/Metrics プリセットと統合テストを拡充し、thumb ターゲットの CI 導入を進める。
