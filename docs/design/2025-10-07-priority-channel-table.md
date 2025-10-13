# Priority Channel Mapping

## 現状サマリ (2025-10-13)
- `PriorityEnvelope::from_system` が全ての `SystemMessage` を Control チャネルで包み、優先度テーブルはコード内に実装済み。
- `SystemMessage::priority()` により Escalate/Failure/Restart/Stop などの順位付けが決定され、mailbox 側の処理順が安定している。
- `PriorityMailboxSpawnerHandle` が制御メッセージを必ず Control チャネルに流すため、Guardian／Scheduler からの経路は統一されている。

## 未解決課題
- [MUST] 優先度テーブルの単体テスト／プロパティテストを追加し、新しい `SystemMessage` 変種追加時のリグレッションを防ぐ。
- [SHOULD] ユーザーメッセージで明示的に Control チャネルを選ぶ API を提供し、緊急メッセージを自前で送信できるようにする（現状は内部 API のみ）。
- [SHOULD] Remote / Cluster 層で Control チャネルが保持されることを検証する統合テストを追加する。
- [MAY] 優先度変更時の監査手順（ドキュメント更新、互換性チェック）をガイド化する。

## 優先アクション
1. `PriorityEnvelope::from_system` の優先度期待値を確認するテストを追加し、CI に組み込む。
2. API 設計を検討し、ユーザーが Control チャネルを利用するためのラッパ（例: `PriorityEnvelope::control_user`) を提案する。
3. Remote メッセージ転送経路でチャネル情報が失われないことを確認するテストケースを追加する。
