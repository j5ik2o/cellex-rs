# Typed Actor DSL：次アクション

## 優先タスク
1. `map_system` の拡張ポイントを公開し、`SystemMessage` をユーザー定義型へ安全にマッピングできるようにする。現状の DSL では型安全な Terminated/Watch 連携が未提供。
2. ライフサイクル Signal（`Started` / `Stopping` / `Stopped` / `Restarting`）を Behavior DSL に組み込む。
3. `Context` / `ActorRef` に watch / unwatch / stop API を公開し、テストを整備する。
4. Stackable `become` API の採用可否を決め、必要なら `BehaviorDirective` を拡張する。

## 参考
- 旧メモは `docs/design/archive/2025-10-07-typed-actor-plan.md` を参照。
