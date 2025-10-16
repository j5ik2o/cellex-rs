# Mailbox Runtime 現状まとめ (2025-10-13)

## 現状サマリ
- `MailboxRuntime` 抽象は `ThreadSafe` / `SingleThread` マーカーで同期境界を切り替え可能になり、Tokio／Local／Embassy それぞれの実装が同じコアを共有している。
- `PriorityMailboxSpawnerHandle` と `QueueMailbox` 系ユーティリティにより、Scheduler 側はファクトリ型に依存せず mailbox を生成できる。
- メタデータは `MetadataStorageMode` を経由してグローバルテーブル管理され、`MessageSender<C>` や `MessageMetadata<C>` が concurrency marker を透過的に扱える状態。

## 未解決課題
- [MUST] `QueueMailboxProducer` / `InternalMessageSender` などで `SingleThread` モードでも不要な `Send + Sync` 制約が残っていないか棚卸しし、必要に応じて API を調整する。
- [MUST] `MailboxOptions` を拡張してバックプレッシャ設定（閾値、動作モード）や RawMutex 選択などの追加パラメータを提供する。
- [MUST] GenericActorRuntime / RuntimeBundle から mailbox/runtime のプリセット（host / embedded / remote）と concurrency モードを選択できる公式 API を整備する。
- [MUST] `embedded_rc` / `embedded_arc` を含むクロスビルドとランタイムテストを CI に追加し、Concurrency 切替のリグレッションを自動検出する。
- [SHOULD] Mailbox メトリクス（enqueue/dequeue、待機長など）を定義し、`PriorityMailboxSpawnerHandle` から一貫して記録できるよう instrumentation を仕上げる。
- [SHOULD] 利用ドキュメント（README / CLAUDE.md）に concurrency マーカーと Mailbox プリセットの運用ガイドを追記する。

## 優先アクション
1. `QueueMailboxProducer` を中心に `SingleThread` 経路の `Send` 境界を精査し、必要な `cfg` や新たな wrapper を追加する。
2. `MailboxRuntime` プリセット（例: `MailboxRuntimePreset::host()`）と GenericActorRuntime への組み込み API を設計し、サンプルコードで利用手順を提示する。
3. `MailboxOptions` 拡張とバックプレッシャのテストケースを追加し、Tokio/Local 双方での動作を確認する。
4. CI に `cargo check -p nexus-actor-embedded-rs --no-default-features --features alloc,embedded_rc` などを追加し、Concurrency モードのビルド保証を自動化する。
