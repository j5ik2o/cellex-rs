# Guardian / Supervisor 設計メモ

## 現状サマリ (2025-10-13)
- `Guardian` と `PriorityScheduler` の統合が完了し、panic 検出や `SystemMessage::Restart/Stop/Escalate` の伝搬経路はコアで動作している。
- `CompositeEscalationSink` により親ガーディアン／カスタムハンドラ／Root イベントリスナーへのルーティングが可能になった。
- `ActorContext` は監視者一覧を保持し、子生成時に親を自動で Watch 登録する挙動がテストで確認済み。

## 未解決課題
- [MUST] `SystemMessage::Terminate` 相当の通知と typed DSL で扱える Terminated イベントの導入（現状は TODO レベル）。
- [MUST] `Context::watch` / `Context::unwatch` / `Context::stop` / `Context::poison` などユーザー向け制御 API を公開し、監視フローをアプリから操作できるようにする。
- [SHOULD] no_std 構成で panic 捕捉ができない場合の代替エラールート（`Result` ベース通知など）を設計する。
- [SHOULD] Typed 層の `map_system` を拡張し、`SystemMessage` を型安全な DSL へマッピングする仕組みを提供する。
- [SHOULD] エスカレーション／ウォッチャ連携をカバーする統合テストを追加し、Guardians 間の階層挙動を自動検証する。

## 優先アクション
1. `SystemMessage::Terminate` と `Terminated` イベントを actor-core に追加し、Guardian/Scheduler 流路を実装する。
2. `Context` / `ActorRef` へ watch/unwatch/stop API を公開し、既存テストを更新する。
3. no_std 時のエラーハンドリング方針をまとめ、必要な抽象（`FailureInfo` 伝搬など）を設計メモに追記する。
