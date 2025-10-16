# ReadyQueue ベースのマルチワーカースケジューラ案 (2025-10-15)

## 背景
- 旧 `PriorityScheduler`（現在は `ReadyQueueScheduler` に統合済み）は単一の `run_forever` ループを Tokio タスク化する構成で、実質的にシングルスレッド実行となっていた。
- `tell` はメールボックスに enqueue した後、その場で `tokio::spawn` を行わずスケジューラに任せたい。protoactor-go / Akka でも同様にメッセージ到着と実行を分離している。
- スループットを確保するためには、スケジューラを Tokio のスレッドプール上で複数ワーカーとして動かす必要がある。

## 最新状況（2025-10-15 更新）
- `PriorityScheduler` は完全に撤廃し、`ReadyQueueSchedulerCore`（旧 PrioritySchedulerCore）が ReadyQueue ベース処理を一本化。
- `SchedulerBuilder::ready_queue()` が既定のビルダーとなり、Tokio / Embassy ラッパーもこの実装を利用する。
- ReadyQueue ワーカーループは `drive_ready_queue_worker` として公開済みで、Tokio ドライバでも複数ワーカーを起動して ReadyQueue を駆動可能。
- 子アクター生成時の命名を `ChildNaming`（`Auto` / `WithPrefix` / `Explicit`）で管理し、RootContext には `spawn_prefix` / `spawn_named` を導入。重複名時は `SpawnError::NameExists` を返す。
- ReadyQueue は signal 通知時にスケジューラ側でキューを更新し、`process_actor_pending` と連携して処理済みアクターの再スケジューリングを制御する。

## 目標（継続）
1. `tell` は常にメールボックスへの enqueue のみを行い、タスク化はスケジューラに委譲する。
2. アクターごとの同時実行は 1 本に保ちつつ、アクター間では複数ワーカーで並行処理できるようにする。
3. protoactor-go / Akka の Dispatcher モデルを参考にし、将来的な Spawn ミドルウェアの差し込みとも整合させる。

## 基本アーキテクチャ

```
tell → Mailbox.enqueue → (Idle → Running 遷移に成功) → ReadyQueue.push(actor_cell_id)
                                                       ↓
                                            Scheduler Worker (tokio::spawn × N)
```

### 1. Mailbox 側
- メールボックスは `scheduler_status` (`Idle` / `Running`) を保持。
- メッセージ到着時に CAS で `Idle` から `Running` へ切り替えられた場合のみ、対応する `ActorCell` を ReadyQueue に追加。
- 既に `Running` であればキューに積むだけで終了。

### 2. ReadyQueue
- 構造体例: `tokio::sync::mpsc::Sender<ActorCellId>`、もしくはロックレスキュー。
- 「未処理メッセージを持つアクター」をワーカー群に配布する待ち行列として機能。
- 一度 push されたアクターは、ワーカー側で処理完了後に `Running → Idle` に戻すまで再度 push されない。

### 3. Scheduler ワーカーループ
- `tokio::spawn` で起動するワーカータスクを `N` 本用意（configurable）。
- 各ワーカーは ReadyQueue から `ActorCellId` を受け取り、`process_messages(actor_cell)` を実行。
- `process_messages` は throughput 件処理したら適宜 `yield_now`。メールボックスが空になったら `Running → Idle` に戻す。
- まだメッセージが残っている（メールボックス側で `Running` のまま）場合は、再度 ReadyQueue に push して次のワーカーに任せる。

### 4. Driver / Runtime 層
- `TokioSystemHandle::start_local` 等でワーカー数に応じて `tokio::spawn` を複数起動。
- `ActorRuntime` へ `spawn_system_task`, `yield_now` などの抽象を追加すると、Tokio / Embedded で共通のインタフェースになる。
- `Spawn` ミドルウェアはアクター登録とメールボックス初期化に集中させる。実際のワーカータスク起動は Driver が一括管理。

## オープン課題
- ReadyQueue の型選定：`mpsc` or lock-free queue。push/pop のコストとバックプレッシャをどう扱うか。
- Throughput 設定との整合：ワーカーが一度に何件処理するか、メトリクスや `yield_now` の挿入ポイント。
- Spawn ミドルウェアとの連携：アクター登録時に ReadyQueue / Mailbox をどう初期化するかを再設計。
- 監視・メトリクス：ワーカー数や ReadyQueue の長さをどう観測するか。
- `SpawnError`/`ChildNaming` を活用した高レベル API（例: 名前→PID ルックアップ）の提供有無を検討。

## 次アクション
1. ReadyQueue ワーカー構成のチューニング方針を決定する（Queue 型・Throughput・メトリクス）。
2. Spawn ミドルウェアとの統合方式を設計し、`ChildNaming` を活かした公開 API を整理する。
3. ReadyQueue の観測ポイント（メトリクス/トレース）を追加し、ワーカー数や滞留長の可視化を進める。
