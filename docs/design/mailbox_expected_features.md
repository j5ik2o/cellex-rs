# メールボックス期待機能と実装状況一覧

**更新日**: 2025-10-27 （SystemMailboxQueue 導入反映）  
**対象**: `modules/actor-core/src/api/mailbox` 系コンポーネント

| 機能 | 説明 | 状態 | 実装 / ギャップ | protoactor-go |
| --- | --- | --- | --- | --- |
| 基本メッセージ投入/受信 | Enqueue → Signal → Dequeue の基本動線。`QueueMailboxCore` が `offer`/`poll` を仲介。 | ✅ | `modules/actor-core/src/api/mailbox/queue_mailbox/core.rs` | ✅ |
| System / User メッセージ優先度 | `PriorityEnvelope` による優先度付けと System 専用レーン。 | ✅ | 優先順位は `modules/actor-core/src/api/mailbox/messages/system_message.rs`。System 専用レーン／予約枠は `SystemMailboxQueue`（`modules/actor-core/src/api/mailbox/queue_mailbox/system_mailbox_queue.rs`）が制御し、`MailboxOptions::default()` で 4 スロットを予約。 | ✅ (systemMailbox) |
| Suspend / Resume 制御 | Suspend でユーザーメッセージ配送停止、Resume で再開。 | ✅ | `ActorCell` が Suspend 状態を保持し、ユーザーメッセージを保留。参照: `modules/actor-core/src/api/actor/tests.rs:710-768`。 | ✅ (`SuspendMailbox`) |
| オーバーフロー処理 | `DropNewest` / `DropOldest` / `Grow` / `Block` の挙動。 | ⚠️ | `modules/utils-core/src/collections/queue/backend/vec_ring_backend.rs`。`Block` が実際には `QueueError::Full` を返すのみ。 | ✅ (`Drop`, `Block`) |
| メトリクス連携 | メールボックス単位のメトリクス記録。 | ✅ | `QueueMailboxCore::record_event`（`modules/actor-core/src/api/mailbox/queue_mailbox/core.rs`）、System 予約枠利用時は `MetricsEvent::MailboxSystemReservedUsed` / `MailboxSystemReservationExhausted` を発火。Suspend/Resume は `MetricsEvent::MailboxSuspended` / `MailboxResumed` で Duration を送出（`metrics_capture_suspend_resume_durations_with_clock` 等で検証）。 | ⚠️（最小限） |
| ReadyQueue 連携 | メールボックスからスケジューラ再登録を行うフック。 | ✅ | `QueueMailboxCore::notify_ready`、`set_scheduler_hook`。 | ✅ (`dispatcher.Schedule`) |
| Throughput / Backpressure ヒント | ReadyQueueCoordinator へ処理数ヒントを提供。 | ⚠️ | トレイト（`modules/actor-core/src/api/actor_scheduler/ready_queue_coordinator/ready_queue_coordinator_trait.rs`）のみ定義。実装での活用は未整備。 | ✅ (`dispatcher.Throughput`) |
| Middleware チェイン | メッセージ処理の前後にフックを挿入。 | ❌ | プランのみ（`docs/design/actor_scheduler_refactor.md` 4.4）。現行コードには未実装。 | ✅ (`MailboxMiddleware`) |
| サスペンション統計・観測 | Suspend 期間・回数などの統計取得。 | ⚠️ | `MetricsEvent::MailboxSuspended` / `MailboxResumed` が回数と Duration（クロック有無に応じて Option）を報告。集計はメトリクスシンク利用側の課題として残存。 | ⚠️（簡易） |
| Stashing / 再投入制御 | 条件付きでメッセージを保留・再投入。 | ❌ | 現行コードに該当機能なし。 | ✅ (Stash) |

## メモ
- System レーンは汎用 `SyncMailboxQueue` を `SystemMailboxQueue` でラップして実現しているため、専用キューの導入済み状態を維持しつつ後方互換は考慮していない。
- System 予約枠は `MailboxOptions::priority_capacity` で調整可能。`None` を指定すると旧来どおり優先レーンなしで動作する。
- Suspend / Resume の詳細分析は `docs/design/suspend_resume_status.md` を参照。ReadyQueue 統合の最新仕様は `openspec/specs/mailbox-suspend-resume/spec.md` に移管済み。
- 2025-10-27 時点で `metrics_capture_suspend_resume_durations_with_clock` / `metrics_omit_duration_when_clock_absent` / `multi_actor_suspend_resume_independent` / `backpressure_resumes_pending_messages` といった ReadyQueueScheduler テストで Suspend/Resume の計測および再投入動作を回帰確認済み。
- 旧実装（nexus-actor-rs）の比較には `docs/design/mailbox_akka_pekko_comparison.md` を参照し、本表は現行実装との差分を追跡する用途とする。
