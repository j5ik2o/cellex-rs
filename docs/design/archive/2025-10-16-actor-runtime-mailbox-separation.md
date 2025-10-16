# ActorRuntime / MailboxRuntime 分離まとめ (2025-10-16)

## 背景
- これまで `ActorRuntime` が `MailboxRuntime` を継承していたため、利用側がメールボックス操作まで同一トレイト境界で扱う必要があり、型制約が肥大化していた。
- `GenericActorRuntime` をランタイムプリセットとして扱いつつ、内部のメールボックス実装は差し替えられる構造にすることが今回の目的だった。

## 達成事項
- `ActorRuntime` は `MailboxRuntime` を関連型として保持するだけの軽量ファサードとなり、コード全体で `MailboxOf<R>` を経由して関連型へアクセスするよう統一した。
- スケジューラ関連 API (`SchedulerBuilder`, `SchedulerSpawnContext`, `ReadyQueueScheduler` など) はメールボックス実装を直接パラメータ化する形へ整理。Tokio / Embassy 向けビルダーも `SchedulerBuilder<M, MailboxRuntime>` 形式で返す。
- `GenericActorRuntime` の受信タイムアウト設定は `ReceiveTimeoutFactoryShared<DynMessage, GenericActorRuntime<R>>` と `ReceiveTimeoutFactoryShared<DynMessage, MailboxOf<R>>` の両方を保持し、`ActorSystem` からはメールボックス型のファクトリが必ずスケジューラへ渡るようになった。
- ドライバ層 (`ReceiveTimeoutDriverShared`) はメールボックス型を返す実装に統一し、`for_mailbox_runtime()` アダプタで双方向の型変換が可能になった。
- `actor-std` / `actor-embedded` 両クレートのスケジューラ・テストを新しい型境界へ移行し、`cargo test --workspace` がグリーンで完了する状態を確認済み。

## 現在の設計ハイライト
- `ActorRuntime` は `mailbox_runtime()` / `mailbox_runtime_shared()` を公開し、利用側は `MailboxOf<R>` を通じてキュー・シグナル・プロデューサを解決する。
- `SchedulerBuilder<M, R>` は「メールボックスランタイム R」を前提とし、Tokio / Embassy 実装は `GenericActorRuntime` を必要としなくなった。
- `ReceiveTimeoutFactoryShared` に `for_runtime_bundle()` / `for_mailbox_runtime()` の両アダプタを備え、ランタイム／メールボックスのどちらの層でも同じファクトリを共有できる。

## 残タスク (2025-10-16 時点)
- ドキュメント整備のみ。設計メモやチュートリアルの図表を最新シグネチャに追随させる。

## 参考
- 実装の詳細は `modules/actor-core/src/runtime/mailbox/traits.rs` や `modules/actor-std/src/scheduler.rs` を参照。
- ベースとした設計は `protoactor-go` の `actor/mailbox` / `actor/actor.go` から Rust 向けに移植したものを起点としている。
