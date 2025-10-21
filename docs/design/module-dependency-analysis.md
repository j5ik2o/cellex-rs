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

### shared 候補の暫定リスト

| 種別 | 現在位置 | 利用箇所 | 移動検討メモ |
| --- | --- | --- | --- |
| `AnyMessage`, `AnyMessageValue` | `api/messaging` → `shared/messaging` | internal の runtime_state, supervision, mailbox など | 2025-10-21 時点で shared へ移設済み。API からの再エクスポートを維持しつつ internal 依存を `crate::shared` 経由に切り替え。 |
| `MessageEnvelope` 系 | `api/messaging` | internal の runtime_state, supervision, mailbox など | type-erasure としてランタイム全体で共通。`PriorityEnvelope` 依存があるため shared 移行は第2段階で検討。 |
| `PriorityEnvelope` | `api/mailbox/messages` | internal/mailbox, ready_queue | メールボックスとスケジューラ双方で必要。 |
| `MailboxFactory` トレイトと関連ハンドル | `api/mailbox` | internal/runtime_state, internal/mailbox | ユーザ API に露出するため、shared に移した後 API で re-export する必要あり。 |
| `MapSystemShared` | `api/actor_system/map_system.rs` | internal/supervision/composite_escalation_sink.rs ほか | system message 変換の共有ハンドル。shared で保管し internal から直接参照可能にする。 |
| `FailureTelemetryShared` 系 | `api/failure/...` | internal/supervision/* | 監視系で両レイヤが利用。shared へ移設し API からは再エクスポート。 |
| `ActorSchedulerHandleBuilder` | `api/actor_scheduler` | internal/runtime_state.rs | ランタイム初期化の共通設定。shared 化しビルダー実装を API 層に露出するか要検討。 |
| `Shared` ラッパ型 | `utils` | api/internal 共通 | 既に utils から直接利用しており shared へ移さない。共有依存の好例として残す。 |
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
