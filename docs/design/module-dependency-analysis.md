# module-dependency-analysis: api/internal 循環依存の現状と整理案

## 概要

`modules/actor-core` では `api/` と `internal/` のレイヤ分離を方針としているが、現状はいくつかの箇所で双方が直接参照し合い、循環依存を形成している。本メモは 2025-10-21 時点の依存状況を洗い出し、今後の shared 抽象化およびディレクトリ再編の前提資料とする。

## 依存グラフの実測結果

- `api::actor::ActorContext::spawn_child` および `RootContext` が `crate::internal::actor::InternalProps` を直接構築し、internal 実装に依存している。  
  - 該当コード: `modules/actor-core/src/api/actor/actor_context.rs:218`、`modules/actor-core/src/api/actor/root_context.rs:60` ほか。
- `internal::actor::InternalProps` や `internal::runtime_state::GenericActorRuntimeState` など internal 側の基盤型が、`crate::api::*` を大量に `use` している。  
  - 例: `modules/actor-core/src/internal/runtime_state.rs:3-7`, `modules/actor-core/src/internal/supervision/composite_escalation_sink.rs:4-15`, `modules/actor-core/src/internal/mailbox/priority_mailbox_builder.rs:3-8`。
- 以上により、`api::actor` → `internal::actor` → `api::actor` の閉路が発生し、静的依存解析では循環が解消できない。

## 問題点

1. **レイヤ境界の侵食**  
   - API 層が internal 実装詳細（`InternalProps` など）を直接生成しており、モジュール分割での責務定義が曖昧。
2. **internal 側の公開 API 依存**  
   - internal 実装が API 型に寄りかかることで、「内部専用にしたい型」が API に引きずられている。再編時に型移動が連鎖しやすい。
3. **shared 抽象の欠如**  
   - 双方で利用する共通型（`AnyMessage`, `MapSystemShared`, `MailboxFactory` 境界など）が `api` 側に置かれており、internal から再利用するたびに依存が発生する。

## 解消方針（ドラフト）

1. **shared レイヤの整備**  
   - `api/internal` の双方で使う抽象を `modules/actor-core/src/shared/` に移動し、`shared` → `utils` だけを参照するよう整理する。  
   - 具体候補: メールボックス境界トレイト、`AnyMessage`/`PriorityEnvelope` ラッパ、`MapSystemShared` などランタイム横断で共通な型。

### shared 候補の棚卸し（2025-10-21）

#### 完了済み（shared へ移設済み）

| 種別 | 旧所在地 | 影響範囲 | 備考 |
| --- | --- | --- | --- |
| `AnyMessage`, `AnyMessageValue` | `api/messaging` | internal runtime_state / mailbox / supervision | `shared/messaging` へ移行済み。API 側は再エクスポートのみ保持。 |
| `MessageEnvelope` | `api/messaging` | internal runtime_state / actor_system / supervision | `shared/messaging` へ移設済み。既存テストも shared 側へ移動。 |
| `PriorityEnvelope` | `api/mailbox/messages` | queue mailbox / ready queue / actor_system | `shared/mailbox/messages` へ移設済み。API からは re-export を廃止。 |

#### shared 移設 TODO（機械実装プラン）

以下は自動化エージェント（Claude Code 等）が直接実施できるレベルまで分解した作業項目です。**再エクスポート禁止**のルールに従い、移設後は API 層からの `pub use` を残さないこと。各タスクは「ファイル移動 → `mod`/`use` 更新 → 旧ファイル削除 → テスト」という順序で実行する。

##### 1. `MapSystemShared`
1. `modules/actor-core/src/api/actor_system/map_system.rs` を `modules/actor-core/src/shared/messaging/map_system.rs` へ移動。モジュール先頭に英語の doc コメントを追加し、`crate::shared::messaging` 参照に変更する。
2. `modules/actor-core/src/shared/messaging.rs` に `pub mod map_system;` と `pub use map_system::MapSystemShared;` を追記。
3. API 側の `mod map_system;` と関連 `pub use` を削除。`modules/actor-core/src/api/actor_system/mod.rs` を更新。
4. `rg "MapSystemShared"` の結果を `crate::shared::messaging::MapSystemShared` へ置換（`internal/actor/internal_props.rs`, `internal/supervision/*`, `modules/actor-std/src/tests.rs` 等）。
5. 元ファイル削除後に `cargo +nightly fmt` → `./scripts/ci-check.sh all` → `makers ci-check -- dylint` を実行。

##### 2. `MailboxFactory` 系抽象
1. 以下のファイルを shared へ移す：
   - `api/mailbox/mailbox_factory.rs` → `shared/mailbox/factory.rs`
   - `api/mailbox/mailbox_handle.rs` → `shared/mailbox/handle.rs`
   - `api/mailbox/mailbox_options.rs` → `shared/mailbox/options.rs`
   - `api/mailbox/mailbox_producer.rs` → `shared/mailbox/producer.rs`
   - `api/mailbox/mailbox_signal.rs` → `shared/mailbox/signal.rs`
2. `modules/actor-core/src/shared/mailbox.rs` を `pub mod factory;` などで更新し、必要なモジュールを公開（`pub use` は付けない）。
3. API 側 `modules/actor-core/src/api/mailbox/mod.rs` から上記ファイルの `mod` 宣言と `pub use` を削除。
4. `rg "api::mailbox"` を `shared::mailbox` に置換。派生クレート (`actor-std`, `actor-embedded`, ベンチ) も含めて修正。
5. 旧ファイルを削除し、フォーマットと CI コマンドを実行。

##### 3. 監視系抽象 (`EscalationSink` など)
1. `modules/actor-core/src/api/supervision/escalation/escalation_sink.rs` を `shared/supervision/escalation_sink.rs` へ移動。`shared/supervision/mod.rs` を新設し `pub mod escalation_sink;` を定義。
2. API 側 `modules/actor-core/src/api/supervision/escalation/mod.rs` は shared の型を `use` するだけに変更し、再エクスポートは行わない。
3. `rg "EscalationSink"` を `crate::shared::supervision::escalation_sink::EscalationSink` に更新（internal supervision 実装、actor-system, tests など）。
4. 旧 API ファイルを削除し、フォーマット＆ CI を実行。

##### 4. `ActorAdapter` ブリッジ抽象
1. `modules/actor-core/src/shared/actor/mod.rs` を新規作成し、`pub trait TypedHandlerBridge<M, AR>` 等 internal が必要とする最小限のメソッドを定義。System メッセージマッパーもここで抽象化する。
2. API の `ActorAdapter` (`api/actor/behavior/actor_adapter.rs`) は shared トレイトの具象実装としてリファクタ。構造体は残しつつ `impl TypedHandlerBridge` を提供。
3. `internal/actor/internal_props.rs` で `crate::api::actor::behavior::ActorAdapter` を直接使用している箇所を shared トレイトに依存するよう変更（ジェネリック引数 `T: TypedHandlerBridge<...>` など）。
4. `MapSystemShared::new` の呼び出しも shared 側へ移設後の API に合わせて更新。
5. 単体テスト (`cargo test -p cellex-actor-core-rs`) を実行し動作確認。

##### 5. Telemetry 共有型
1. `modules/actor-core/src/api/failure/failure_telemetry.rs` から共有 struct (`FailureTelemetryShared`, `FailureTelemetryBuilderShared`, `FailureTelemetryContext`) を切り出し、`shared/failure/telemetry.rs` を新規作成。
2. `shared/failure/mod.rs` を追加し、`pub mod telemetry;` を設定。必要に応じて `pub use telemetry::{FailureTelemetryShared, ...};` を記述。
3. API 側ファイルは DSL・構成体のみ残し、共有型は `use crate::shared::failure::telemetry::...;` を利用。
4. `rg "FailureTelemetryShared"` で import を更新し、旧定義を削除。
5. フォーマット & CI を実行。

##### 6. その他（テスト専用依存）
- `PriorityActorRef` や `MessageSender` など internal テストが依存する API 型は、shared 移設ではなくテストダブルを internal に導入する案を検討。必要に応じて専用タスクを別途起票（現時点では保留）。

各タスク完了後の共通作業:

1. `cargo +nightly fmt`
2. `./scripts/ci-check.sh all`
3. `makers ci-check -- dylint`

失敗したコマンドがあればログを記録し、インポート漏れや `mod` 宣言忘れを解消してから再実行すること。
2. **API 層の境界インタフェース化**  
   - `ActorContext::spawn_child` など API 側で internal 型を直接返さず、共有インタフェース経由で internal 実装に委譲する。  
   - 例: `InternalProps` を生成するファクトリトレイトを shared に置き、internal 側が実装する。
3. **internal → API 依存の分離**  
   - internal 側で API 型が必要な箇所にはアダプタ層を設け、API 型が exposing している共有抽象だけを参照するよう変更する。

## 次のアクション

1. shared レイヤ候補の棚卸しと `module-dependency-analysis` 更新（本メモの継続管理）。  
2. `ActorContext` / `RootContext` から `InternalProps` への直接呼び出しを包む境界トレイトを設計。  
3. internal 各モジュールで `crate::api` 依存箇所を列挙し、shared 抽象またはアダプタ経由にリファクタする計画を立案。  
4. shared 抽象導入後に `cargo modules` などのツールで再度依存グラフを出力し、循環が解消されたことを検証する。
