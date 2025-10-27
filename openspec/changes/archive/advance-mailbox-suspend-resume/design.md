# Design: Mailbox Suspend/Resume Completion

## Current State
- `ActorCell` が `ActorCellState::Suspended` を保持し、ユーザーメッセージを保留キューに退避する実装は完了している。
- `ReadyQueueScheduler` は `ActorInvokeOutcome` を `InvokeResult` に変換し、`ResumeCondition::After` / `WhenCapacityAvailable` の一部経路で再投入できるテストを通過している。
- ただし `ReadyQueueCoordinator` へは一時的なテスト用 API (`inject_invoke_result_for_testing`) を用いており、本番フローでの `InvokeResult::Suspended` 伝播は未統合。
- メトリクス (`MetricsEvent::MailboxSuspended` / `MailboxResumed`) は定義済みながら、Suspend 期間を `SuspensionClockShared` から取得する処理や no_std 環境でのフォールバックが未完成。

## Remaining Work
1. **メトリクス拡張**
   - `SuspensionClockShared` が有効な場合に期間を測定し、`MetricsEvent` に付与する。
   - no_std / null clock の場合は回数のみ集計し、テストで検証できるようフィクスチャを整備。

2. **ReadyQueueCoordinator 連携**
   - `ReadyQueueSchedulerCore` から `ReadyQueueCoordinator` へ実際の `handle_invoke_result` を配線し、Suspend 中の Mailbox を再投入しないよう制御。
   - `ResumeCondition::ExternalSignal` のキー管理を `ReadyQueueCoordinator` 共有レジストリへ整合させる。

3. **テスト拡充**
   - 複数アクターの並列 Suspend/Resume、バックプレッシャ解除、シグナル経由 Resume など、プランで要求されている統合シナリオを網羅。
   - メトリクスのタイミング検証、および `SuspendReason` ごとのハンドリングを確認する。

4. **ドキュメント更新**
   - `docs/design/mailbox_suspend_resume_plan.md` と関連設計資料を最新化し、残タスクの完了を明記。

## Risks & Mitigations
- Suspend 中のメッセージ蓄積が多い場合、メモリ使用量が増加する -> テストで極端なケースを検証し、必要に応じてバックプレッシャとの連携仕様を文書化。
- `SuspensionClockShared` が利用できない環境での条件分岐が複雑化する恐れ -> clock 有無を分岐するヘルパを導入し、単体テストで no_clock ケースをカバー。

