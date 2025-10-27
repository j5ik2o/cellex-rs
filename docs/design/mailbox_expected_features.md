# メールボックス期待機能と実装状況一覧

**更新日**: 2025-10-27  
**対象**: `modules/actor-core/src/api/mailbox` 系コンポーネント

| 機能 | 説明 | 状態 | 実装 / ギャップ | protoactor-go |
| --- | --- | --- | --- | --- |
| 基本メッセージ投入/受信 | Enqueue → Signal → Dequeue の基本動線。`QueueMailboxCore` が `offer`/`poll` を仲介。 | ✅ | `modules/actor-core/src/api/mailbox/queue_mailbox/core.rs` | ✅ |
| System / User メッセージ優先度 | `PriorityEnvelope` による優先度付け。専用キュー・予約枠は未導入。 | ⚠️ | 優先順位は `modules/actor-core/src/api/mailbox/messages/system_message.rs`。System 専用バッファは未実装。 | ✅ (systemMailbox) |
| Suspend / Resume 制御 | Suspend でユーザーメッセージ配送停止、Resume で再開。 | ✅ | `ActorCell` が Suspend 状態を保持し、ユーザーメッセージを保留。参照: `modules/actor-core/src/api/actor/tests.rs:710-768`。 | ✅ (`SuspendMailbox`) |
| オーバーフロー処理 | `DropNewest` / `DropOldest` / `Grow` / `Block` の挙動。 | ⚠️ | `modules/utils-core/src/collections/queue/backend/vec_ring_backend.rs`。`Block` が実際には `QueueError::Full` を返すのみ。 | ✅ (`Drop`, `Block`) |
| メトリクス連携 | メールボックス単位のメトリクス記録。 | ✅ | `QueueMailboxCore::record_event`（`modules/actor-core/src/api/mailbox/queue_mailbox/core.rs`）。 | ⚠️（最小限） |
| ReadyQueue 連携 | メールボックスからスケジューラ再登録を行うフック。 | ✅ | `QueueMailboxCore::notify_ready`、`set_scheduler_hook`。 | ✅ (`dispatcher.Schedule`) |
| Throughput / Backpressure ヒント | ReadyQueueCoordinator へ処理数ヒントを提供。 | ⚠️ | トレイト（`modules/actor-core/src/api/actor_scheduler/ready_queue_coordinator/ready_queue_coordinator_trait.rs`）のみ定義。実装での活用は未整備。 | ✅ (`dispatcher.Throughput`) |
| Middleware チェイン | メッセージ処理の前後にフックを挿入。 | ❌ | プランのみ（`docs/design/actor_scheduler_refactor.md` 4.4）。現行コードには未実装。 | ✅ (`MailboxMiddleware`) |
| サスペンション統計・観測 | Suspend 期間・回数などの統計取得。 | ❌ | `MailboxSuspensionState` 相当の構造体が未実装。旧実装は `docs/sources/nexus-actor-rs/...` を参照。 | ⚠️（簡易） |
| Stashing / 再投入制御 | 条件付きでメッセージを保留・再投入。 | ❌ | 現行コードに該当機能なし。 | ✅ (Stash) |

## メモ
- Suspend / Resume の詳細分析は `docs/design/suspend_resume_status.md` を参照。
- 旧実装（nexus-actor-rs）の挙動と比較する際は `docs/design/mailbox_akka_pekko_comparison.md` を併せて確認すること。
