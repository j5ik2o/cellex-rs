# Guardian / Supervisor 拡張：次アクション

## 優先タスク
1. `SystemMessage` に Terminate 相当を追加し、Guardian/Scheduler 経路へ伝搬させる。現状の enum には `Stop` までしか存在しないため（`modules/actor-core/src/runtime/mailbox/messages.rs` を確認）。
2. `Context` / `ActorRef` に watch / unwatch / stop 等の制御 API を公開し、テストを整備する。現行の typed DSL では未公開。
3. no_std 構成での panic 代替経路を設計し、エラー通知を Result ベースで扱えるようにする。
4. Guardian 階層の統合テストを追加し、エスカレーションとウォッチャ連携を自動検証する。

## 参考
- 旧メモは `docs/design/archive/2025-10-07-guardian-supervisor-plan.md` を参照。
