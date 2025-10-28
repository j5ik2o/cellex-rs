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
| Suspend/Resume | `ActorCell` がサスペンド状態と保留キューを保持するよう改修済み（2025-10-27）。 | 仕様どおり停止・再開できる。残課題はメトリクス／バックプレッシャ統合。 |
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

### 4.1 新規構造体と状態遷移

```rust
pub(crate) enum ActorCellState {
  Running,
  Suspended,
  Stopped,
}

pub struct ActorCell<MF, Strat> {
  state: ActorCellState,
  pending_user_envelopes: VecDeque<PriorityEnvelope<AnyMessage>>,
  suspension_clock: SuspensionClockShared,
  suspend_started_at: Option<u64>,
  last_suspend_nanos: Option<u64>,
  total_suspend_nanos: u128,
  suspend_count: u64,
  resume_count: u64,
  // 既存フィールド...
}
```

- Suspend 時は `ActorCellState::Suspended` へ遷移し、ユーザーメッセージを `pending_user_envelopes` に退避。`SuspensionClockShared` が提供されていれば、サスペンド開始時刻を記録。  
- Resume で `ActorCellState::Running` に戻し、退避済みメッセージを ReadyQueue へ戻す（`collect_envelopes` 内で drain）。同時に、Suspend 期間を算出してメトリクスへ送出（クロック未設定の場合は `None`）。

### 4.2 メールボックス処理の流れ

1. `QueueMailboxCore::try_send_mailbox`  
   - Suspend 中も enqueue 自体は許可（Backpressure 制御は別途）。  
   - ReadyQueue 通知は既存どおり。実際の配送は `ActorCell` 側で抑制。

2. `QueueMailboxRecv` / `QueueMailboxCore::try_dequeue_mailbox`  
   - Suspend 中でも dequeuing は行うが、ユーザーメッセージは `ActorCell` によって保留キューへ移される。  
   - System メッセージは常に処理対象。

3. `ActorCell::dispatch_envelope`  
   - SystemMessage::Suspend → `transition_to_suspended()` を呼び出し、以降のユーザーメッセージを保留。  
   - SystemMessage::Resume → `transition_to_running()` を呼び出し、保留分を順次再投入。

4. `ReadyQueueCoordinator`  
   - `InvokeResult::Suspended` を受けて ReadyQueue から対象を除外し、`ResumeCondition`（After / ExternalSignal / WhenCapacityAvailable）に応じて `ReadyQueueHandle` を通じて再登録する。

### 4.3 メトリクス

- `MetricsEvent::MailboxSuspended` / `MailboxResumed` を追加し、Suspend/Resume 回数と最新／累積の Duration（計測可能な環境のみ）を記録。ReadyQueueScheduler テストでモッククロック／クロック無しの双方を検証。  
- no_std 環境では `SuspensionClockShared::null()` を用いたフォールバックで回数のみを記録。

---

## 5. 実装ステップ

1. **基盤整備（完了 2025-10-27）**  
   - `ActorCellState` と `pending_user_envelopes` を導入し、`ActorCell` が Suspend/Resume を制御。  
   - ReadyQueue とのインタラクションは既存 API を維持。

2. **SystemMessage ハンドリング（完了）**  
   - `ActorCell::dispatch_envelope` で Suspend/Resume の状態遷移を実装。  
   - System メッセージの通知経路は従来どおり。

3. **デキュー処理の更新（完了）**  
   - `collect_envelopes` が保留キューとの drain を担当。  
   - Resume 後に保留分が処理されることを保証。

4. **メトリクス & イベント（完了 2025-10-27 / 2025-10-27 再確認）**  
   - `MetricsEvent::MailboxSuspended` / `MailboxResumed` が Suspend/Resume 回数と Duration を記録し、モッククロックおよび `SuspensionClockShared::null()` の両ケースをテストで検証済み。  
   - no_std 環境向けフォールバックは `SuspensionClockShared::null()` で提供し、Duration フィールドは `None` となる。

5. **テスト追加（完了 2025-10-27 / 2025-10-27 拡張）**  
   - Suspend ブロックと Resume 後の再開に加え、複数アクター並列 Suspend/Resume・外部シグナル・バックプレッシャ条件を網羅する ReadyQueueScheduler テストを追加。

6. **ドキュメント更新（継続）**  
   - 本ドキュメント、`mailbox_expected_features.md` などを最新構成へ更新。  
   - 将来的に `InvokeResult::Suspended` 統合方針を追記。

---

## 6. 関連タスク

| 優先度 | タスク | 担当 | 備考 |
| --- | --- | --- | --- |
| P0 | `ActorCellState` / 保留キュー導入 | ✅ | 実装済み（2025-10-27）。 |
| P0 | Suspend/Resume のテスト整備 | ✅ | `test_suspend_accumulates_messages_until_resume` など更新。 |
| P1 | メトリクスイベント追加とテレメトリ調整 | ✅ | Suspend/Resume Duration を含むメトリクスを実装し、クロック有・無のテストを追加。 |
| P1 | ReadyQueueCoordinator 連携の強化 | ✅ | `InvokeResult::Suspended` の本経路を整備し、After / ExternalSignal / WhenCapacityAvailable の再登録を確認。 |
| P1 | ドキュメント更新 | ✅ | 本ドキュメントおよび `mailbox_expected_features.md` を更新。 |

---

## 7. リスクと検討事項

- Suspend 中にメッセージが大量に蓄積される場合のバックプレッシャ制御。  
- ReadyQueueCoordinator 側で Suspend された index を適切に除外できているか（テストでは確認済みだが継続監視）。  
- Embedded/no_std 環境での `Instant` 利用（必要に応じて抽象化）。  
- `MailboxProducer` が Suspend 状態を認識し、Backpressure エラーを返すかどうかの仕様整理。

---

## 8. 完了条件

- Suspend/Resume が仕様どおり動作し、`docs/design/suspend_resume_status.md` のギャップが解消されていること。  
- 新設テストが成功し、既存テストに回帰がないこと。  
- メトリクスで Suspend/Resume のイベントが観測できること（`metrics_capture_suspend_resume_durations_with_clock` など）。  
- `docs/design/mailbox_expected_features.md` の Suspend/Resume 行が ✅ へ更新されていること。
