# Suspend/Resume 実装計画

**作成日**: 2025-10-27  
**対象**: `modules/actor-core/src/api/mailbox` および関連コンポーネント  
**参照ドキュメント**:  
- `docs/design/suspend_resume_status.md`  
- `docs/design/mailbox_feature_comparison.md`  
- `docs/design/mailbox_expected_features.md`  
- `docs/sources/nexus-actor-rs/modules/actor-std/src/actor/dispatch/mailbox/default_mailbox.rs`

---

## 1. ゴール

1. `SystemMessage::Suspend` 受信時にユーザーメッセージ配送を停止し、`SystemMessage::Resume` 受信時に安全に再開できるようにする。  
2. サスペンド状態・期間・再開回数などの統計を取得し、メトリクス経由で観測できるようにする。  
3. ReadyQueue / Guardian / メトリクスとの連携が破綻しないよう、既存の API 境界を維持する。  
4. 既存テストとの互換性を保ちつつ、新しい挙動を検証するユニットテスト・統合テストを追加する。

---

## 2. 現状の課題

| 項目 | 現状 | 影響 |
| --- | --- | --- |
| Suspend/Resume | `SystemMessage::Suspend/Resume` は列挙されているが、`QueueMailbox` / `ActorCell` が処理を停止しない。 | Suspend 後もユーザーメッセージが処理され、制御不能。 |
| 状態管理 | サスペンド状態を保持する構造が存在しない。 | 停止/再開状態の追跡やメトリクス記録が不可能。 |
| ReadyQueue連携 | Suspend 中も ReadyQueue に再登録され続ける。 | 無駄なスケジューリングとフェアネス低下。 |
| テスト | `test_typed_actor_stateful_behavior_with_suspend_resume` などを Suspend ブロック仕様に合わせて更新する必要がある。 | リグレッション検知ができない。 |

---

## 3. 参照実装（protoactor-go / nexus-actor-rs）

| 機能 | protoactor-go | nexus-actor-rs |
| --- | --- | --- |
| サスペンド状態フラグ | ✅ `mailbox.isSuspended` | ✅ `MailboxSuspensionState` |
| サスペンド期間・統計 | ⚠️ 簡易的 | ✅ `total_duration`, `resume_events` |
| ReadyQueue からの除外 | ✅ dispatcher 再登録を停止 | ✅ suspend 中は userMailbox を処理しない |
| メトリクス | ⚠️ 最小限 | ✅ Dropwizard/Prometheus 連携 |

---

## 4. 提案アーキテクチャ

### 4.1 新規構造体

```rust
pub struct MailboxSuspension {
  flag: Flag,                     // サスペンド状態
  since: SharedMutex<Option<Instant>>,
  resume_events: AtomicU64,
  total_nanos: AtomicU64,
}
```

- `QueueMailboxCore` に `suspension: MailboxSuspension` を追加。  
- `suspend()` / `resume()` / `is_suspended()` / `record_resume()` メソッドを提供。  
- `Flag` は既存の `cellex_utils_core_rs::sync::Flag` を利用。  
- `SharedMutex` は `SpinSyncMutex` / `CriticalSection` 等、ターゲット依存の抽象を使用。

### 4.2 メールボックス処理の流れ

1. `QueueMailboxCore::try_send_mailbox`  
   - Suspend 中も enqueue 自体は許可（Backpressure 制御は別途）。  
   - ただし enqueue 後に ReadyQueue へ通知する際、すでに Suspend 状態なら再登録を抑制し、Resume まで保留。

2. `QueueMailboxRecv` / `QueueMailboxCore::try_dequeue_mailbox`  
   - Suspend 中にユーザーメッセージを取り出さない。  
   - System メッセージは常に取り出す（Suspend/Resume 評価用）。

3. `ActorCell::dispatch_envelope`  
   - SystemMessage::Suspend → `QueueMailboxCore::suspend()` を呼び出し、ReadyQueueCoordinator に `SuspendReason::UserDefined` を通知。  
   - SystemMessage::Resume → `QueueMailboxCore::resume()` を呼び出し、ReadyQueueCoordinator に `InvokeResult::Suspended` 解消を通知。

4. `ReadyQueueCoordinator`  
   - Suspend 通知を受け取ったら対象 index を除外。Resume で再登録。

### 4.3 メトリクス

- `MetricsEvent::MailboxSuspended` / `MailboxResumed` を追加。  
- Resume 時に `duration` を計測し、合計時間を記録。

---

## 5. 実装ステップ

1. **基盤整備**  
   - `MailboxSuspension` 構造体を `modules/actor-core/src/api/mailbox/queue_mailbox` 配下に追加。  
   - `QueueMailboxCore` にフィールド追加と suspend/resume メソッドを実装。

2. **SystemMessage ハンドリング**  
   - `ActorCell::dispatch_envelope` で Suspend/Resume の分岐を追加。  
   - Suspend 時は `ReadyQueueCoordinator::unregister()`、Resume 時は `register_ready()` を呼び出す。

3. **デキュー処理の更新**  
   - Suspend 中はユーザーメッセージを `Pending` として扱い、System メッセージのみ処理。  
   - Resume 時に保留分を再登録。

4. **メトリクス & イベント**  
   - `MetricsEvent` に Suspend/Resume を追加。  
   - 再開時間を `total_nanos` に積算。

5. **テスト追加**  
   - Suspend 後にユーザーメッセージが処理されないことを確認するユニットテスト。  
   - Resume 後に処理が再開されること、メトリクスが記録されることを確認。

6. **ドキュメント更新**  
   - `docs/design/mailbox_expected_features.md` の Suspend/Resume 状態を更新。  
   - `docs/design/actor_scheduler_refactor.md` の前提条件に「Suspend/Resume 実装済み」を明記。

---

## 6. 関連タスク

| 優先度 | タスク | 担当 | 備考 |
| --- | --- | --- | --- |
| P0 | `MailboxSuspension` 実装と `QueueMailboxCore` 組み込み |  | protoactor-go / nexus-actor-rs を参考。 |
| P0 | `ActorCell` の SystemMessage ハンドリング更新 |  | Suspend/Resume 時の ReadyQueue 連携。 |
| P0 | ReadyQueueCoordinator への通知 API 拡張 |  | Suspend index の管理。 |
| P1 | メトリクスイベント追加とテレメトリ調整 |  | `MetricsEvent` の追加、MetricsSink テスト。 |
| P1 | テストケース整備（単体 + 統合） |  | Suspend → Resume サイクル、保留メッセージの扱い。 |
| P1 | ドキュメント更新 |  | 設計メモ／期待機能一覧の更新。 |

---

## 7. リスクと検討事項

- Suspend 中にメッセージが大量に蓄積される場合のバックプレッシャ制御。  
- ReadyQueueCoordinator 側で Suspend された index を適切に除外できているか。  
- Embedded/no_std 環境での `Instant` 利用（必要に応じて抽象化）。  
- `MailboxProducer` が Suspend 状態を認識し、Backpressure エラーを返すかどうかの仕様整理。

---

## 8. 完了条件

- Suspend/Resume が仕様どおり動作し、`docs/design/suspend_resume_status.md` のギャップが解消されていること。  
- 新設テストが成功し、既存テストに回帰がないこと。  
- メトリクスで Suspend/Resume のイベントが観測できること。  
- `docs/design/mailbox_expected_features.md` の Suspend/Resume 行が ✅ へ更新されていること。
