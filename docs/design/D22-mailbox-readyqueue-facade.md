# Mailbox / ReadyQueue ファサード再設計メモ

## 背景
- 現行コードは protoactor-go および旧 nexus 実装を丁寧に Rust 化しているが、責務が暗黙に分散し理解が難しい。
- QueueMailbox はシグナル通知と ReadyQueue へのブリッジに特化し、メッセージ実行ループは ActorCell 内に収斂している。
- ReadyQueueScheduler は実質的にスケジューラ中核だが、Dispatcher や Invoker に相当する概念が明示されていない。
- 旧 DefaultMailbox が持っていた Suspend/Resume、middleware、詳細メトリクスなどの機能が部分的に未移植で、再統合の設計指針が必要。

## 現状の責務マッピング
| レイヤ | 主な型 | 現行責務 |
| --- | --- | --- |
| Mailbox | `QueueMailbox`, `QueueMailboxProducer` | enqueue/notify、ReadyQueue への再登録、メトリクス enqueue 記録 |
| Invoker 相当 | `ActorCell` | 優先度付きバッチ dequeue、System/User メッセージ処理、サスペンド判定、失敗伝播 |
| Dispatcher 相当 | `ReadyQueueScheduler` + `ReadyQueueWorkerImpl` | Ready index の管理、ActorCell の処理・再待機、ワーカ駆動 |
| ランタイム駆動 | `ActorSystemRunner`, `runtime_driver.rs` | Tokio タスク生成、ワーカ数調整、shutdown 協調 |

## 課題
- Mailbox ↔ Scheduler ↔ Invoker の境界が暗黙で API から意図が読み取りづらい。
- Suspend/Resume や middleware といった旧機能が ActorCell／ReadyQueue に散らばり、拡張ポイントが見えない。
- ReadyQueueScheduler がファサードとして外部に見えているが、内部構造が把握しづらく、dispatcher を別モジュールに抽出した方が説明しやすい。
- メトリクス／バックプレッシャ設定などの TODO が複数ドキュメントに散在している。

## 目標アーキテクチャ
### コンポーネント構成
1. **Mailbox Core**: QueueMailbox を中心に enqueue・シグナル・ReadyQueueHook を担当。データ構造 + イベント発火装置に徹する。
2. **Scheduler Facade**: ReadyQueueScheduler を facade として整理し、内部に以下のサブコンポーネントを保持。
   - `MessageDispatcher`: ワーカ生成・スケジュール要求（Tokio など各ランタイムへ `spawn`）。
   - `MessageInvoker`: ActorCell からメッセージ処理を呼び出す抽象インタフェース。ActorCell 実装をデフォルトとして提供。
3. **Execution Runner**: ActorSystemRunner / runtime drivers は Dispatcher 経由でワーカを駆動し、shutdown・観測ポイントを管理。

### イベントフロー（案）
1. Producer が QueueMailbox へ enqueue → signal.notify → ReadyQueueHook.notify_ready。
2. ReadyQueueScheduler が index を ready キューへ配置し、Dispatcher に処理要求を発行。
3. Dispatcher はランタイムタスクを生成し、Invoker(ActorCell) の `run`/`process` を呼ぶ。
4. Invoker は Envelope バッチ処理・サスペンド制御・ガーディアン連携を担い、結果に応じて Scheduler へ再登録。

## フェーズ別タスク案
### Phase 1: 概念抽出とインタフェース定義
- `MessageInvoker` トレイトを導入し、ActorCell のメッセージ処理メソッドを順次移植。
- ReadyQueueScheduler 内に Dispatcher 構造体を導入し、ワーカ駆動と `notify_ready` ハンドルを整理。
- QueueMailbox/ActorCell/ReadyQueue 間のイベントフローを図解し README 更新。

### Phase 2: 旧機能の再統合
- Suspend/Resume, middleware, dequeue metrics を Invoker/Dispatcher レイヤへ再配置。
- `MailboxOptions` 拡張（バックプレッシャ閾値、通知ハンドル差し替え）。
- メトリクス sink を enqueue/dequeue 双方で一貫して記録。

### Phase 3: ランタイム統合と観測強化
- Dispatcher を介したワーカ数チューニング API を公開し、`ReadyQueueWorker` との接続を一本化。
- 測定ポイント（滞留長・再スケジュール回数・処理レイテンシ）を整備し、既存メトリクス sink へ送出。
- Tokio / Embassy 向け runtime driver の共通ロジックを抽出し、Facade からの呼び出しを簡素化。

## 既存 TODO の取り込み
- `D14-mailbox-runtime-next-actions.md` の優先タスク（Send/Sync 境界精査、MailboxOptions 拡張、プリセット API、クロスビルド CI、メトリクス整備）を Phase 2/3 のサブタスクとして明示的に管理。
- `D13-ready-queue-next-actions.md` のワーカチューニング、Spawn ミドルウェア統合、観測ポイント強化を Dispatcher / Facade のロードマップに統合。
- `docs/design/archive/2025-10-13-mailbox-runtime-status.md` の進捗項目（QueueMailboxProducer の SingleThread 対応、metrics 仕上げ）を継続課題として Phase 2 に含める。

## 成果物イメージ
- `MessageInvoker` / `MessageDispatcher` の新モジュール設計書。
- QueueMailbox と ReadyQueueScheduler の API ドキュメント更新。
- ランタイムごとの driver 実装ガイド（Tokio, Embassy, Local）。

## オープン課題
- Suspend/Resume を ReadyQueue レベルで止めるか、Invoker 内で完結させるかの設計判断。
- Middleware の API を `PriorityMailboxSpawnerHandle` に再導入するか、Invoker へ委譲するかの方針決定。
- MetricsSink を lock-free に保ちながら両方向（enqueue/dequeue）で記録する実装戦略。
- Embedded モードでの Dispatcher 抽象をどう標準化するか（`task::spawn_local` 非依存な設計）。

## 次のアクション
1. 本メモをもとにアーキ議論を実施し、Phase 1 の優先順位を確定。
2. `MessageInvoker` トレイト導入のドラフト PR を作成し、ActorCell から責務を切り出す。
3. ReadyQueueScheduler 内部に Dispatcher 構造体雛形を追加し、Tokio driver との接続コードを段階的に移管。
