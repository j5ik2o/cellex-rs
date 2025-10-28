# Tasks: implement-block-overflow-policy

Option A（Mailbox を AsyncQueue ベースへ刷新）の実装タスク。上から順に実施する。

## Prerequisites

- [ ] Proposal / Design / Spec Delta が承認済み
- [ ] `openspec validate implement-block-overflow-policy --strict` を事前実行して初期状態を把握
- [ ] Mailbox 関連の既存テスト（同期 API 前提）を洗い出し

## Phase 1: トレイトと基盤の async 化

### Task 1.1: MailboxQueueBackend トレイトの async 化

- [ ] `modules/actor-core/src/api/mailbox/queue_mailbox/backend.rs` で `MailboxQueueBackend` に `#[async_trait(?Send)]` を付与し、`offer` / `poll` / `close` を `async fn` 化
- [ ] `len` / `capacity` / `set_metrics_sink` のシグネチャを維持しつつ、必要に応じて内部で `ArcShared` を扱うよう調整
- [ ] 既存の実装がすべてコンパイルエラーになることを確認（影響範囲の洗い出し）

### Task 1.2: System Mailbox 実装の更新

- [ ] `SystemMailboxLane` を実装している型（例: `system_mailbox_queue.rs`）を async トレイトに合わせて更新
- [ ] 既存の同期ロジックを async 化し、即時完了する処理は `async { ... }` で包んで即座に `Ok` を返す

## Phase 2: UserMailboxQueue の AsyncQueue 化

### Task 2.1: 内部構造の刷新

- [ ] `UserMailboxQueue` を `AsyncQueue` ベースへ置き換え
  - `ArcShared<SpinAsyncMutex<SyncAdapterQueueBackend<VecRingBackend>>>` を生成
  - `AsyncQueue::new_mpsc` で共有キューを構築
- [ ] `offer` / `poll` / `close` / `set_metrics_sink` を `async fn` として再実装
- [ ] `OverflowPolicy` ごとの動作（Drop/Grow など）が従来どおりになるよう再確認

### Task 2.2: 単体テスト追加

- [ ] `modules/actor-core/src/api/mailbox/queue_mailbox/user_mailbox_queue/tests.rs` を async テストへ書き換え
- [ ] Block ポリシーの待機 → poll 後に再開 → close 時に `QueueError::Disconnected` を返すケースを追加
- [ ] Drop/Grow ポリシーが回帰していないことを確認

## Phase 3: QueueMailbox Core/Handle/Producer の async 化

### Task 3.1: QueueMailboxCore の更新

- [ ] `QueueMailboxCore` の `try_send_mailbox` / `try_send` / `try_dequeue_mailbox` / `close` を `async fn` 化
- [ ] System queue とユーザー queue の双方で `await` を適切に挿入
- [ ] メトリクス送出・閉塞フラグ処理が従来通り動作することを確認

### Task 3.2: 外部インターフェースの async 化

- [ ] `QueueMailbox` / `QueueMailboxHandle` / `QueueMailboxProducer` の公開 API（`offer` / `poll` / `flush` / `stop` 等）を async 化
- [ ] 呼び出し元（スケジューラや Mailbox 利用コード）のビルドエラーを解消し、必要なら Transitional ラッパーを提供

### Task 3.3: 信号とメトリクスの整合

- [ ] `MailboxSignal` 呼び出しとの整合を確認（必要なら async 対応または即時呼び出しを維持）
- [ ] メトリクス用ロックが async 化によってデッドロックしないことを確認

## Phase 4: テスト & ドキュメント

### Task 4.1: 統合テスト更新

- [ ] `queue_mailbox/tests.rs` など Mailbox 経路の統合テストをすべて async 版へ移行
- [ ] Block ポリシーで producer を複数 spawn し、FIFO 順に再開されるテストを追加
- [ ] 既存の Drop/Grow テストを async に移行し、動作検証を実施

### Task 4.2: ドキュメント整備

- [ ] `docs/design/mailbox_expected_features.md` の Block 項目を ✅ へ更新し、簡潔な説明を追記
- [ ] Mailbox API の使用例（docs/examples, README 等該当箇所）を async 版へ差し替え

## Phase 5: 最終検証

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `cargo +nightly fmt`
- [ ] `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi`
- [ ] `cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi`
- [ ] `./scripts/ci-check.sh all`
- [ ] `openspec validate implement-block-overflow-policy --strict`

すべてのタスク完了後、チェックボックスを更新して完了状態を明示すること。
