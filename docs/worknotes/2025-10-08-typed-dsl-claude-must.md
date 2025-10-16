# Claude Code 向け Typed DSL MUST 作業指示

## 背景と目的
- [`docs/design/D5-typed-actor-next-actions.md`](docs/design/D5-typed-actor-next-actions.md) の TODO #2 / #3（TypedActorAdapter + map_system 経路の強化）が継続課題。
- 現行の typed API は `modules/actor-core/src/api/actor/` 配下で提供され、`Behavior<U, R>` / `Context<'_, '_, U, R>` / `Props<U, R>` / `ActorAdapter<U, R>` が実装済み。
- Guardian / Scheduler は `ActorAdapter::create_map_system()` が返すクロージャを介して制御メッセージを配送するため、ユーザー定義 enum への射影やテスト整備が未完のまま残っている。

## 現状概要（要参照）
- `Behavior<U, R>` / `ActorAdapter<U, R>` / `Props<U, R>` は `modules/actor-core/src/api/actor/behavior.rs` と `props.rs` に実装済み。
- `ActorAdapter::create_map_system()` は `MessageEnvelope::<U>::System(sys)` を返す固定クロージャのままで、ユーザーが独自 enum を用意する余地がない。
- Guardian / Scheduler は `map_system` を以下の経路で利用:
  - [`ReadyQueueSchedulerCore::spawn_actor`](modules/actor-core/src/runtime/scheduler/ready_queue_scheduler.rs:85)
  - [`ActorCell::register_child_from_spec`](modules/actor-core/src/runtime/scheduler/actor_cell.rs:299)
  - [`Guardian::register_child`](modules/actor-core/src/runtime/guardian/core.rs:41)
  - [`CompositeEscalationSink`](modules/actor-core/src/runtime/supervision/composite_sink.rs) を通じて親 Guardian へ連鎖させるハンドリング
- 統合テストは `modules/actor-core/src/api/actor/tests.rs` の `typed_actor_system_handles_user_messages` などが中心で、SystemMessage 経路の typed 変換テストが不足している。

## 作業タスク

### Task 1 — ActorAdapter の柔軟性向上
1. `modules/actor-core/src/api/actor/behavior.rs` 内の `ActorAdapter<U, R>` を拡張し、ユーザーが `SystemMessage` をアプリ固有 enum へマッピングできるようにする。
2. `handle_user` / `handle_system` は既存の動作を維持しつつ、`system_handler` が提供された場合に `MessageEnvelope::<U>::System` 以外の経路も許容する（例: enum 変換）。
3. `ActorAdapter::create_map_system()` をジェネリックにし、呼び出し側から `Arc<dyn Fn(SystemMessage) -> MessageEnvelope<U>>` を注入できるよう API を見直す。
4. `Props::with_behavior_and_system`（modules/actor-core/src/api/actor/props.rs:110）で新しい `map_system` 生成ロジックを受け取り、既定値は従来の `MessageEnvelope::System` を利用する。

### Task 2 — map_system 生成の型安全化
1. `MapSystemShared` の既定クロージャとユーザー提供クロージャを切り替えられるようにし、`MessageEnvelope::<U>` 以外のコンテナでも動作可能にする。
2. `Arc<dyn Fn(SystemMessage) -> MessageEnvelope<U> + Send + Sync>` インターフェースは維持しつつ、Stateless/Stateful それぞれのサンプル（enum 包含など）を `docs/design/D5-typed-actor-next-actions.md` に追記する。
3. Guardian / Scheduler の呼び出し箇所で追加の型情報が不要であることを確認し、`SystemMessage::Escalate` などの優先度保証テストを `ActorAdapter` のユニットテストとして追加する。

### Task 3 — 統合テスト拡張
1. `modules/actor-core/src/api/actor/tests.rs` の `#[cfg(test)]` セクションに以下のケースを追加:
   - `test_system_stop_transitions_behavior`: `SystemMessage::Stop` をカスタム enum 経由で受け取り、`Behavior::stopped()` へ遷移すること。
   - `test_watch_unwatch_routed_via_map_system`: `SystemMessage::Watch`/`Unwatch` が型付きイベントへ変換され、`Context::register_watcher` の結果が検証できること。
   - `test_stateful_behavior_handles_failure`: Stateful Behavior が Failure/Escalate を processed event として受け取り、内部状態を更新すること。
2. テストでは `PriorityActorRef::try_send_control_with_priority` と `MessageEnvelope::<U>` を用いて制御メッセージの優先度が保持されているか確認する。
3. `TestMailboxRuntime` を用いた end-to-end シナリオを追加し、Guardian 経由で Escalate が親に届くことを確認する。

## 実装メモ
- `ActorAdapter` が `ArcShared` を保持する際、`Send + Sync` 制約と `Clone` 実装（MailboxRuntime 要件）を満たす必要がある。
- `SystemMessage` の優先度は [`SystemMessage::priority`](modules/actor-core/src/runtime/mailbox/messages.rs:46) で決定済み。Typed 層で再定義しない。
- `Context<'r, 'ctx, U, R>` のライフタイム制約を満たすため、クロージャ引数は `for<'r, 'ctx>` で定義すること。

## テストと品質保証
- 実装後に以下コマンドを **必ず** 実行して成功を確認:
  - `cargo +nightly fmt`
  - `cargo clippy --workspace --all-targets --all-features`
- `cargo test -p cellex-actor-core-rs`
- `cargo test -p cellex-actor-core-rs --features std`
- 追加テストでは panic / unwrap を避け、`Result` は `expect` で理由を明示。

## ドキュメント更新
- 作業完了後に [`docs/design/D5-typed-actor-next-actions.md`](docs/design/D5-typed-actor-next-actions.md) の TODO #2 / #3 を完了扱いへ更新し、関連知見があれば新たな節を追加。
- `progress.md` に進捗ログを追記。

## 完了条件（Definition of Done）
1. `ActorAdapter` と `map_system` 改修がマージされ、Typed DSL が SystemMessage を型安全に扱える。
2. 上記テスト群が追加され、`cargo test` 系がすべて成功。
3. ドキュメント・進捗ログが更新され、次タスクへ引き継げる状態を整備。
4. Lint（clippy）とフォーマット（rustfmt）で警告が出ない。

以上のガイドに基づき、Claude Code で実装を進めてください。
