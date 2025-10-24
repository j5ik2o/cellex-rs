# ADR-001: ActorScheduler リファクタリングにおけるコンポーネント命名ポリシー

## ステータス

提案中

## コンテキスト

ActorScheduler のリファクタリング（Phase 0-4）において、以下の課題が存在します：

### 現在の問題点

1. **責務と命名の不一致**
   - `ReadyQueueWorker` が実際には「ワーカループ駆動」だけでなく「ActorCell実行」も担当
   - `ReadyQueueContext` が「コンテキスト」という抽象的な名前で具体的な責務が不明瞭

2. **参照実装との用語の齟齬**
   - protoactor-go では `Dispatcher` が複数の責務を持つ
   - Akka では `Dispatcher` と `Executor` が明確に分離
   - 現行実装ではこれらの概念が欠落

3. **拡張性への懸念**
   - 新しいコンポーネント（WorkerExecutor, MessageInvoker 等）を導入する際の命名基準が不明確
   - ドキュメントと実装で異なる名称が使われる可能性

### 制約条件

- 外部公開 API（`ReadyQueueScheduler`、`ActorScheduler` トレイト）は後方互換性を維持
- Phase 1-4 で段階的にリファクタリングするため、命名は一貫性を保つ必要がある
- protoactor-go / Akka / Erlang といった参照実装との整合性を重視

### 前提条件

- 設計ドキュメント `docs/design/actor_scheduler_refactor.md` に基づく
- Phase 4 で最終的な命名整理を行う（`ActorSchedulerFrontend` への改名是非等）

## 決定

### 選択した解決策

以下の命名ポリシーを採用します：

#### 1. コア責務語彙の定義

| 語彙 | 意味 | 採用理由 |
|------|------|----------|
| **Coordinator** | 調整・調停 | Ready queue の調整、シグナル管理、再登録制御を担当。protoactor-go の `MailboxScheduler` に相当 |
| **Executor** | 実行 | ランタイムタスク生成、ワーカ駆動を担当。Akka の `ExecutorService` に相当 |
| **Invoker** | 呼び出し | メッセージ実行ループ。protoactor-go の `MessageInvoker` と同名 |

#### 2. コンポーネント命名表

| 新名称 | 旧名称（検討時） | 参照実装での対応 | 主な責務 |
|--------|------------------|------------------|----------|
| **ReadyQueueCoordinator** | ReadyQueueDriver | protoactor-go `MailboxScheduler` / Akka `Dispatcher` | Ready queue の調整・シグナル管理・再登録制御 |
| **WorkerExecutor** | MessageDispatcher | protoactor-go `Dispatcher`(タスク実行) / Akka `ExecutorService` | ランタイムタスク生成・ワーカ駆動・Invoker 呼び出し |
| **MessageInvoker** | MessageInvoker | protoactor-go `MessageInvoker` | メッセージ実行・Suspend/Resume 判定・Guardian 連携 |
| **ReadyQueueScheduler** | (変更なし) | - | 外部API窓口（フロント層） |

#### 3. 命名時の原則

1. **責務の明確性**: 名前から責務が推測できること
2. **参照実装との整合**: protoactor-go / Akka の用語に可能な限り合わせる
3. **`Driver` の不使用**: ハードウェア抽象と混同を避けるため、`Driver` は使用しない
4. **一貫性**: 設計ドキュメント・コード・テスト・ADRで同一名称を使用

#### 4. Phase 4 での最終判断

- `ReadyQueueScheduler` を `ActorSchedulerFrontend` へ改称するか否かは Phase 4 で判断
- Phase 0-3 では `ReadyQueueScheduler` のまま維持し、内部実装のみ変更

### 代替案

#### 代替案 1: すべて `Dispatcher` で統一

- **概要**: `ReadyQueueDispatcher`, `WorkerDispatcher`, `MessageDispatcher` のように `Dispatcher` を接頭辞として統一
- **利点**: Akka / protoactor-go との用語的な一貫性
- **欠点**: `Dispatcher` が複数の意味を持ち、責務の境界が曖昧になる
- **不採用の理由**: `Dispatcher` は Akka では「ワーカスレッド管理」、protoactor-go では「メッセージ配送」を指し、混乱を招く

#### 代替案 2: `Driver` を使用

- **概要**: `ReadyQueueDriver`, `WorkerDriver` のように `Driver` を採用
- **利点**: 汎用的な命名で分かりやすい
- **欠点**: ハードウェア抽象（デバイスドライバ）との混同、本設計の責務（調整・実行）と齟齬
- **不採用の理由**: ドキュメントレビューで「`Driver` はハードウェア層を連想させる」との指摘

#### 代替案 3: Akka 準拠の命名

- **概要**: `Dispatcher` (Coordinator相当), `ExecutorService` (Executor相当), `ActorCell` (Invoker相当)
- **利点**: Akka の知見を持つ開発者にとって直感的
- **欠点**: Rust のエコシステムや protoactor-go との整合性が低い
- **不採用の理由**: cellex-rs は protoactor-go を主な参照実装としており、用語の統一を優先

## 結果

### 利点

1. **責務の明確化**: Coordinator/Executor/Invoker の語彙により、各コンポーネントの役割が一目瞭然
2. **参照実装との整合**: protoactor-go の `MessageInvoker`、Akka の `ExecutorService` との対応が明確
3. **ドキュメント整合性**: 設計ドキュメント・PlantUML図・コード・ADRで統一された用語
4. **拡張性の向上**: 新しいコンポーネント追加時の命名基準が明確

### 欠点・トレードオフ

1. **学習コスト**: 既存の `ReadyQueueContext` から `ReadyQueueCoordinator` への変更を理解する必要
2. **移行期の混在**: Phase 1-3 では旧実装と新実装が混在し、一時的に複数の命名が共存
3. **Akka との差異**: Akka の `Dispatcher` とは異なる命名のため、Akka 経験者には違和感の可能性

### 影響を受けるコンポーネント

- **ReadyQueueScheduler**: 内部で `ReadyQueueCoordinator` を生成・保持
- **ReadyQueueContext**: Phase 1 で `ReadyQueueCoordinator` へ段階的に移行
- **ReadyQueueWorker**: Phase 2A で `WorkerExecutor` へ分離
- **ActorCell**: Phase 2B で `MessageInvoker` へメッセージ実行ロジックを移譲

### 移行計画

#### Phase 0 (現在)
- 命名ポリシーの確定と ADR 承認
- 設計ドキュメント・PlantUML図での命名統一

#### Phase 1
- `ReadyQueueCoordinator` トレイトと実装を新規作成
- `ReadyQueueContext` から責務を段階的に移譲
- Feature flag `new-scheduler` で切り替え可能に

#### Phase 2A
- `WorkerExecutor` トレイト実装
- `ReadyQueueWorkerImpl` から責務を移譲

#### Phase 2B
- `MessageInvoker` トレイト実装
- `ActorCell` からメッセージ実行ロジックを分離

#### Phase 4
- `ReadyQueueScheduler` → `ActorSchedulerFrontend` への改称是非を判断
- 旧実装の削除と命名の最終整理

### 検証方法

#### メトリクス
- コードレビュー時の命名に関する指摘数（目標: 0件）
- ドキュメントと実装の命名一致率（目標: 100%）

#### 成功基準
- Phase 1 完了時点で `ReadyQueueCoordinator` 関連のテストが全て通過
- 設計ドキュメント・PlantUML図・コードで同一用語が使用されている
- レビュアーから「命名が直感的」とのフィードバックを得る

#### テスト方法
- 命名整合性チェックスクリプト（grep で用語の一貫性を検証）
- ドキュメントレビュー（最低2名以上の承認）

## 参照

### 関連ドキュメント

- [actor_scheduler_refactor.md](../design/actor_scheduler_refactor.md) - 全体設計
- [scheduler_component_mapping.puml](../design/scheduler_component_mapping.puml) - 責務マッピング図
- [scheduler_dependency_graph.md](../design/scheduler_dependency_graph.md) - 依存関係グラフ

### 外部リンク

- [protoactor-go Dispatcher](https://github.com/asynkron/protoactor-go/tree/dev/actor)
- [Akka Dispatcher](https://doc.akka.io/docs/akka/current/typed/dispatchers.html)
- [Erlang OTP gen_server](https://www.erlang.org/doc/man/gen_server.html)

## メタ情報

- **作成日**: 2025-10-22
- **最終更新日**: 2025-10-22
- **作成者**: Claude Code
- **レビュアー**: (未定)
- **関連 Issue/PR**: (未定)
- **対象フェーズ**: Phase 0

---

## 更新履歴

| 日付 | 変更内容 | 変更者 |
|------|----------|--------|
| 2025-10-22 | 初版作成 | Claude Code |
