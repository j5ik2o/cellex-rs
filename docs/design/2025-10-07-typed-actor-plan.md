# Typed Actor 設計メモ

## 現状サマリ (2025-10-13)
- `Behavior<U, R>`・`ActorAdapter`・`TypedContext` の主要 API は actor-core に統合済みで、Ask/Respond も typed センダー経由で完結する。
- `map_system` は `ActorAdapter::create_map_system()` を通じて Guardian/Scheduler へのブリッジを提供し、既存の制御メッセージは typed DSL から受け取れる。
- Typed 版 Ask (`ask_with_timeout`, `AskFuture`) やステートフル `Behavior::setup` など基礎的な DSL は利用可能。

## 未解決課題
- [MUST] `SystemMessage` をユーザー定義 enum／ドメイン型に変換できる `map_system` 拡張ポイントを提供し、受信側で安全に分岐できるようにする。
- [MUST] `Behavior` に追加ライフサイクルイベント（`Started` / `Stopping` / `Stopped` / `Restarting`）を Signal として流す。
- [MUST] `Context::watch` / `unwatch` / `stop` など監視 API を typed 層で公開し、watcher 登録を DSL から操作できるようにする。
- [SHOULD] `BehaviorDirective` に stackable な `become` 系 API（`become_stacked` / `unbecome`）を追加し、複雑な状態遷移を表現できるようにする。
- [SHOULD] Typed DSL のドキュメント／サンプルコードを刷新し、map_system のカスタマイズ手順やテストベストプラクティスを明記する。

## 優先アクション
1. `TypedSystemEvent`（仮称）を定義し、`map_system` の戻り値として利用できる拡張ポイントを実装する。
2. 監視 API を typed Context / ActorRef に追加し、Watch/Unwatch 経路の統合テストを整備する。
3. 追加ライフサイクル Signal を Behavior DSL に組み込み、既存テストを更新する。
