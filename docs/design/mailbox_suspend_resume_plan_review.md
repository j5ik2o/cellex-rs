# Suspend/Resume 実装計画レビュー

**レビュー実施日**: 2025-10-27  
**レビュー対象**: `docs/design/mailbox_suspend_resume_plan.md`  
**レビュアー**: Codex (GPT-5)

---

## サマリー

- 総合評価: 4.5 / 5.0 （最新実装で主要懸念は解消済み。将来のメトリクス拡張・InvokeResult 統合が残課題）
- 良い点: ゴールが明確、既存ドキュメントの参照が整理されている、ReadyQueue/メトリクス/テストを意識したステップ分解がある。
- 主な懸念: ADR-002 との整合、Mailbox レイヤでの Suspend 判定の実現性、no_std 向けの時間計測と同期プリミティブの選定が未確定。

---

## ブロッカー (Must Fix)

- 主要な Must Fix 指摘は最新実装で解消済み（ActorCell が状態・保留キューを保持し、Resume 時に ReadyQueueHook を介して再通知）。
- 今後の重点項目はメトリクス統計の拡張（Suspend 期間の計測）、および ReadyQueueCoordinator との InvokeResult 統合（ステートレスな連携）の仕上げ。

---

## 重要な改善提案 (Should Fix)

- **同期プリミティブと共有抽象の明確化**  
  - 計画中の `SharedMutex` はコードベースに存在しない名称。`cellex_utils_core_rs::sync::Shared` と `SpinSyncMutex` など既存プリミティブの組み合わせを使うのか、新規導入なのかを明示すること。  
  - 将来 `LocalMailbox` 実装にも適用できるよう、`MailboxSuspension*` ではなく `ActorSuspensionState` 等の命名と抽象化を検討すること。

- **no_std 向けの時間計測戦略**  
  - `Instant` は `thumbv6m-none-eabi` で利用不可。`SuspendMetricsClock` のような trait を用意し、`std` 環境では `Instant`、`no_std` 環境ではダミー記録もしくは `embedded-time` 等の抽象を差し込む設計を追加してほしい。  
  - 計測自体を feature gated にする場合も、API 表面をどうするかを文書化する。

- **バックプレッシャと enqueue 仕様の整理**  
  - Suspend 中に enqueue を許可するとバッファ無限化リスクがある。少なくとも Phase 0 では「Suspend 中は enqueue を許可するが ReadyQueue には再登録しない」「bounded queue overflow 時は Backpressure エラーを返す」などのポリシーを明文化すること。

- **MetricsEvent の拡張要件を具体化**  
  - `MetricsEvent::MailboxSuspended` / `MailboxResumed` は実装済み（Suspend/Resume 回数をカウント）。今後は Suspend 期間計測や exporter 設計を整理する。  
  - `total_nanos` 相当の統計導入タイミングを roadmap に反映する。

---

## 補足提案 (Nice to Have)

- `MailboxSuspension` を導入する場合でも、テストや監視のために ActorCell から問い合わせ可能な API を提供（例: `fn suspension_snapshot(&self) -> MailboxSuspensionStats`）。
- `InvokeResult::Suspended { resume_on }` に紐づく Resume 条件（外部シグナル、タイムアウト等）を Phase 0 でどう扱うかを追記する。
- 文書末尾の完了条件に、ADR の整合性チェックや `cargo make coverage` によるベンチ確認を追加しておくと移行の抜け漏れ防止になる。

---

## テスト観点

- 単体テスト: `ActorCell::dispatch_envelope` に対する Suspend/Resume の基本ケース（システムメッセージ優先、二重 Suspend/Resume の冪等性、Suspend 中の Stop/Escalate 取り扱い）。
- 統合テスト: ReadyQueueCoordinator を含む suspend サイクル、bounded mailbox での overflow、複数アクター同時 Suspend の再開順序。既存テスト `test_typed_actor_stateful_behavior_with_suspend_resume` が Suspend ブロック仕様をカバーすることを確認する。
- ベンチ: suspend check 追加によるホットパス劣化が 5% 以内であることを `cargo bench actor_cell::dispatch` 系で確認する計画を明記する。

---

## 次のアクション

1. ADR-002 方針への修正 → 完了済み（ActorCell が Suspend/Resume を管理）。
2. ReadyQueue ハンドシェイク → Resume 時の ReadyQueueHandle 再通知を実装。InvokeResult 統合は Phase 2 で追跡。  
3. メトリクス拡張 → イベント追加済み。期間計測/no_std クロックは別タスクとして管理。  
4. テストカバレッジ → Suspend/Resume テスト更新済み。並行ケースは継続課題。

---

「MUST」を解消しない限り、実装に着手すべきではないと判断しました。上記の修正が計画に反映された段階で、改めてレビューしたいと思います。
