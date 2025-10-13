# 継続作業プロンプト（次セッション引き継ぎ用）

## 現状
- リファクタ済みで全テスト成功（`cargo test --workspace`）。
- 抽象の意味整理が完了。
  - 旧 `MailboxRuntime` → `ActorRuntime`（trait）
  - 旧 `ActorRuntimeBundle` → `RuntimeEnv<R>`（struct）、`RuntimeEnvCore<R>` も導入
- コアは `RuntimeEnv<R: ActorRuntime>` が R を保持し、`ActorRuntime` を実装して内包 R に委譲。
- 代表参照:
  - `modules/actor-core/src/api/actor/system.rs:27` ActorSystem は `R: ActorRuntime` を受けつつ内部は `RuntimeEnv<R>`
  - `modules/actor-core/src/api/actor/system.rs:342` `impl<R> ActorRuntime for RuntimeEnv<R>`
  - `type RuntimeParam<R> = RuntimeEnv<R>` が複数箇所に存在（例: `modules/actor-core/src/api/actor/context.rs:36`）

## 目的
- 依存方向を反転できる設計へ移行準備。「ActorRuntime 内に RuntimeEnv を内包できる」構造へ段階移行。
- 直ちに完全反転はせず、ブリッジトレイトで滑らかに切り替え可能にする。

## 方針（段階移行・低リスク）
1) ブリッジ導入（RuntimeEnvAccess）
   - 例：
     - `trait RuntimeEnvAccess: ActorRuntime { type Base: ActorRuntime; fn env(&self) -> &RuntimeEnv<Self::Base>; }`
     - 初期実装：`impl<R: ActorRuntime + Clone + 'static> RuntimeEnvAccess for RuntimeEnv<R> { type Base = R; fn env(&self) -> &RuntimeEnv<R> { self } }`
   - 目的：既存の `RuntimeEnv<R>` も、将来の「内包型R」も、同じ API（`env()`）で扱えるようにする。

2) 消費側の境界を `RuntimeEnvAccess` に寄せる
   - `ActorSystem` の `where` を `R: ActorRuntime + Clone + 'static` から `R: ActorRuntime + RuntimeEnvAccess + Clone + 'static` へ。
   - `InternalActorSystem<..., RuntimeEnv<R>, ...>` を `InternalActorSystem<..., R, ...>` に変更。
   - `type RuntimeParam<R> = RuntimeEnv<R>` を `type RuntimeParam<R> = R` に変更。
     - 対象ファイル：
       - `modules/actor-core/src/api/actor/context.rs:36`
       - `modules/actor-core/src/api/actor/actor_ref.rs:12`
       - `modules/actor-core/src/api/actor/props.rs:23`
       - `modules/actor-core/src/api/messaging/message_envelope.rs:538`
       - `modules/actor-core/src/api/actor/root_context.rs:12`
   - 上記で `RuntimeEnv` 固有 API が必要な箇所は `r.env().<method>` に差し替え（例：`priority_mailbox_spawner`、`scheduler_builder`、`receive_timeout_factory`）。

3) スケジューラ系のジェネリクス切替
   - `SchedulerBuilder<DynMessage, RuntimeEnv<R>>` → `SchedulerBuilder<DynMessage, R>`。
   - ビルダー/スケジューラ実装で `R: RuntimeEnvAccess` を参照し、`env()` 経由で必要リソースへアクセス。
   - 主な対象：`modules/actor-core/src/runtime/scheduler/` 配下（`actor_scheduler.rs`, `priority_scheduler.rs`, `immediate_scheduler.rs` など）と、利用側の `api/actor/system.rs`。

4) 既存 API の互換維持
   - `RuntimeEnv<R>` は当面存置（`impl ActorRuntime` も維持）。
   - 将来、「TokioActorRuntime」等の具体ランタイムへ `RuntimeEnvAccess` を実装し、`RuntimeEnv` 内包構造へ差し替え。

5) ドキュメント/用語統一
   - docs 配下の `MailboxRuntime` 記述を `ActorRuntime` / `RuntimeEnv` に置換。
   - 設計メモの依存説明を「R: ActorRuntime」「RuntimeEnv<R> = 束」に更新。

## 受け入れ条件
- `cargo test --workspace` 成功。
- RP2040/RP2350 クロスチェック（任意）：
  - `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi`
  - `cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi`
- `cargo clippy --workspace --all-targets` で新規警告を増やさない。

## 参考ファイル
- `modules/actor-core/src/api/actor/system.rs`
- `modules/actor-core/src/api/actor/{context.rs, actor_ref.rs, props.rs, root_context.rs}`
- `modules/actor-core/src/api/messaging/message_envelope.rs:538`
- `modules/actor-core/src/runtime/scheduler/*`

## 注意
- mod.rs 禁止（2018 モジュール）。
- rustdoc は英語、それ以外は日本語コメント。
- 破壊的変更は許容だが、段階移行でテストグリーンを維持。

## 実行コマンド例
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo +nightly fmt`

