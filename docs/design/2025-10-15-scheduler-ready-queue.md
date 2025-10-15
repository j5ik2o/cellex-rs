# ReadyQueue ベースのマルチワーカースケジューラ案 (2025-10-15)

## 背景
- 旧 `PriorityScheduler`（現在は `ReadyQueueScheduler` に統合済み）は単一の `run_forever` ループを Tokio タスク化する構成で、実質的にシングルスレッド実行となっていた。
- `tell` はメールボックスに enqueue した後、その場で `tokio::spawn` を行わずスケジューラに任せたい。protoactor-go / Akka でも同様にメッセージ到着と実行を分離している。
- スループットを確保するためには、スケジューラを Tokio のスレッドプール上で複数ワーカーとして動かす必要がある。

## 現状整理と変更点
- `ReadyQueueSchedulerCore` を分離し、`ReadyQueueScheduler` が `ArcShared<spin::Mutex<ReadyQueueState>>` を共有してアクターの ready 状態を管理する。
- `SchedulerBuilder::ready_queue()` は `ReadyQueueScheduler` を返すように更新済み。Tokio / Embassy のラッパーも同スケジューラを内包する。
- ReadyQueue は signal 通知時にスケジューラ側でキューを更新し、`process_actor_pending` との組み合わせで処理済みアクターの再スケジューリングを制御する。

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

## 変更インパクト
- ReadyQueue スケジューラの内部構造を「単一ループ」から「ReadyQueue + ワーカー群」へ再設計する必要がある。
- メールボックスには `scheduler_status` と ReadyQueue への通知処理を追加する。
- `ActorSystemRunner::run_forever` は ReadyQueue ワーカー起動のラッパへ置き換える。
- テストではワーカー数 1 を指定すれば従来挙動と同等になり、Embedded 向けの動作も維持可能。

## オープン課題
- ReadyQueue の型選定：`mpsc` or lock-free queue。push/pop のコストとバックプレッシャをどう扱うか。
- Throughput 設定との整合：ワーカーが一度に何件処理するか、メトリクスや `yield_now` の挿入ポイント。
- Spawn ミドルウェアとの連携：アクター登録時に ReadyQueue / Mailbox をどう初期化するかを再設計。
- 監視・メトリクス：ワーカー数や ReadyQueue の長さをどう観測するか。

## 次アクション
1. Prototype: ReadyQueue + ワーカーマルチ化した `ReadyQueueScheduler` の PoC を作成。
2. Mailbox 側に `scheduler_status` を導入し、Idle/Running ガードを実装。
3. Driver 側（TokioSystemHandle 等）をワーカー数設定に対応させる。
4. Spawn ミドルウェアの再導入（protoactor-go の `defaultSpawner` 相当）を検討。
