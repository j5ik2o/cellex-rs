# Proposal: Mailbox Suspend/Resume Completion

## Context
- リポジトリの `docs/design/mailbox_suspend_resume_plan.md` では Suspend/Resume 実装の段階的なロードマップを定義済み。
- 2025-10-27 時点の状況から、`ActorCell` 側のサスペンド制御や ReadyQueueScheduler テストは整備済みで、おおよそ 75% の進捗。
- 残タスクはメトリクスの充実、ReadyQueueCoordinator への正式な `InvokeResult::Suspended` 連携、複数シナリオのテスト拡充、設計ドキュメント更新など。

## Goal
Suspend/Resume フローを完成させ、メトリクス・キュー協調・ドキュメントを含む最終的な品質を確保する。

## Scope
- メトリクス計測 (`SuspensionClockShared`) の本格運用
- ReadyQueueCoordinator 連携と外部 Resume 通知
- 並列シナリオやバックプレッシャのテスト拡充
- 主要ドキュメント更新

