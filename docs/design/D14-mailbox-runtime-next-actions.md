# Mailbox Runtime 状況：次アクション

## 優先タスク
1. `QueueMailboxProducer` などで `SingleThread` 構成でも不要な `Send + Sync` 境界が付与されていないか精査し、必要なら条件付き実装へ調整する（現状 `queue_mailbox.rs` で無条件 `unsafe impl Send/Sync`）。
2. `MailboxOptions` を拡張し、バックプレッシャ設定や通知ハンドルのカスタマイズを提供する。
3. GenericActorRuntime から mailbox/runtime プリセット（host / embedded / remote）と concurrency モードを選択できる API を提供する。
4. `embedded_rc` / `embedded_arc` を含むクロスビルドとランタイムテストを CI に追加する。
5. Mailbox メトリクスの定義とドキュメント整備を進め、 `PriorityMailboxSpawnerHandle` から一貫して記録できるようにする。

## 参考
- 旧メモは `docs/design/archive/2025-10-13-mailbox-runtime-status.md` を参照。
