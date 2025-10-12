# ActorRuntimeBundle / ActorSystem 新API導入計画 (2025-10-12)

## 背景と判断
- 2025-10-11 までの検討では既存の `ActorSystem<U, R, Strat>` や `ActorRuntimeBundle<R>` を直接書き換える案だったが、影響範囲が広く段階的移行が困難であることが判明した。
- 既存 API を利用しているテスト・サンプル・設計メモが多数存在し、一括置換は PR 分割とレビュー負担を増大させる。
- そのため既存実装は維持しつつ、新しい実装一式を `new_runtime/`（仮称）配下に並行導入する方針へ転換する。
- 旧 API からの移行は、新 API の安定化後に段階的に行う。両者が併存する期間は長期にわたり得るため、明確な名前空間と橋渡しレイヤを設ける必要がある。

## ゴール
1. `core` クレート内に `new_runtime/` 名前空間を用意し、新しい `ActorSystem` とランタイムバンドル群を定義する。
2. 既存の `ActorSystem` / `ActorRuntimeBundle` を触らずに、新 API 側で必要な構造・トレイト・ビルダーを実装する。
3. 現行コードとの橋渡しを可能にする互換レイヤ（薄いアダプタ）を提供し、段階的に呼び出し元を移行できるようにする。
4. API サーフェス、モジュール構成、テスト戦略を明文化し、Claude Code / Claude 4.5 でも迷わず実装できるドキュメントを整備する。

## 参照資料
- `docs/design/2025-10-04-shared-abstraction-plan.md`
- `docs/design/2025-10-06-mailbox-runtime-plan.md`
- `docs/design/2025-10-11-runtime-bundle-plan.md`
- 既存実装参考: `docs/sources/protoactor-go`（Go 実装からの移植時に参照）

## 新API設計概要
### コア trait と命名方針
> **名称検討:** `ActorRuntimeBundle` という語は「複数の Runtime を束ねる」印象を与えるが、実際には Mailbox/Scheduler/TimeoutDriver などのツール群をまとめる役割である。候補として `ActorRuntime`（バンドルを省いた名称）への改名を検討する。フェーズAで trait を実装する際に、命名の最終判断を行い、コードとドキュメントで統一すること。

```rust
/// Shared runtime bundle entry point for the new API surface.
pub trait NewActorRuntimeBundle: Clone + Send + Sync + 'static {
  type MailboxRuntime: NewMailboxRuntime;
  type SchedulerBuilder: NewSchedulerBuilder<Self::MailboxRuntime>;

  fn mailbox_handle_factory(&self) -> Arc<dyn NewMailboxHandleFactory<Self::MailboxRuntime>>;
  fn scheduler_builder(&self) -> Arc<Self::SchedulerBuilder>;
  fn receive_timeout_factory(&self) -> Option<ReceiveTimeoutFactoryShared<DynMessage, Self::MailboxRuntime>>;
  fn metrics_sink(&self) -> Option<MetricsSinkShared>;
  fn root_event_listener(&self) -> Option<FailureEventListener>;
  fn root_escalation_handler(&self) -> Option<FailureEventHandler>;
  fn runtime_parts(&self) -> RuntimeParts<Self::MailboxRuntime>;
  fn extensions(&self) -> &Extensions;
}

pub struct NewInternalActorSystem<M, B, Strat = AlwaysRestart>
where
  M: Element,
  B: NewActorRuntimeBundle,
{
  inner: InternalActorSystem<DynMessage, B::MailboxRuntime>,
  _marker: PhantomData<(M, Strat)>,
}

pub struct NewActorSystem<U, B, Strat = AlwaysRestart>
where
  U: Element,
  B: NewActorRuntimeBundle,
  Strat: GuardianStrategy<DynMessage, B::MailboxRuntime>,
{
  inner: NewInternalActorSystem<DynMessage, B::MailboxRuntime, Strat>,
  shutdown: ShutdownToken,
  extensions: Extensions,
  _marker: PhantomData<U>,
}
```

### バンドル具象型
- `HostTokioBundleNew`: ホスト環境 (Tokio runtime) 向けの既定構成。
- `EmbeddedBundleNew`: 組み込み（`no_std` + Embassy/Tokio）向け。初期段階では `Noop` 実装でテストベースを用意し、後続で最適化。
- `TestHarnessBundle`: ユニットテスト専用の簡易構成。既存の `TestMailboxRuntime` を再利用。

### 既存 API との橋渡し
- `compat::from_legacy_bundle(legacy: Arc<ActorRuntimeBundleLegacy>) -> HostTokioBundleNew` のようなアダプタを用意する。
- 互換レイヤでは最低限の変換（MailboxRuntime、Scheduler）を行い、既存の設定構造体を再利用できるようにする。
- 新 API の導入初期はこのアダプタを通じてテストやサンプルを動作させ、徐々にネイティブな新バンドル構築へ移行する。

## 型・エイリアス一覧（新設予定）
| 識別子 | 種別 | 役割 | 備考 |
| --- | --- | --- | --- |
| `NewActorRuntimeBundle` | trait | 新 API におけるランタイムバンドルインターフェース | 旧 `ActorRuntimeBundle` と併存 |
| `NewMailboxRuntime` | trait | Mailbox 実装の共通インターフェース | `protoactor-go` の Mailbox を参考に定義 |
| `NewMailboxHandleFactory<R>` | trait | Mailbox ハンドル生成の抽象 | `RuntimeParts` 経由でハンドルを提供 |
| `NewSchedulerBuilder<R>` | trait | スケジューラ生成のビルダー | `R: NewMailboxRuntime` に紐づく |
| `RuntimeParts<R>` | struct | `InternalActorSystem` に渡す構成要素の束 | MailboxRuntime・Scheduler・TimeoutFactory を保持 |
| `NewInternalActorSystem<M, B, Strat>` | struct | 既存 `InternalActorSystem` との橋渡し層 | 現状は `DynMessage` 固定で運用 |
| `NewActorSystem<U, B, Strat>` | struct | 新 API の `ActorSystem` | `Async` 共有モデルに最適化 |
| `HostTokioBundleNew` | struct | Tokio ホスト向けバンドル | `new_runtime/host_tokio.rs` に配置 |
| `EmbeddedBundleNew` | struct | 組み込み向けバンドル | `new_runtime/embedded.rs` に配置 |
| `TestHarnessBundle` | struct | テスト専用バンドル | `new_runtime/test_harness.rs` に配置 |

## モジュール構成案（core クレート）
- `src/new_runtime.rs`: ルートモジュール。公開 API と再エクスポートを記述。
- `src/new_runtime/actor_system.rs`: `NewActorSystem` と関連設定型。
- `src/new_runtime/bundle.rs`: `NewActorRuntimeBundle` trait と共通ヘルパ。
- `src/new_runtime/host_tokio.rs`: `HostTokioBundleNew` 実装。
- `src/new_runtime/embedded.rs`: `EmbeddedBundleNew` 実装（暫定 `Noop` ベース）。
- `src/new_runtime/test_harness.rs`: テスト用バンドル。
- `src/new_runtime/mailbox.rs`: Mailbox 周辺トレイトと型定義。
- `src/new_runtime/scheduler.rs`: スケジューラビルダー関連。
- `src/new_runtime/compat.rs`: 既存 API とのアダプタ。

※ `mod.rs` は使用せず、Rust 2018 構成に従う。

## 内部構成と橋渡し方針
- `NewActorSystem` 直下に新しい内部構造体 `NewInternalActorSystem<M, B, Strat>` を設け、既存 `InternalActorSystem<DynMessage, B::MailboxRuntime>` をフィールドとして保持する薄いラッパ層を実装する（現状 `M = DynMessage` で運用）。
- `NewInternalActorSystem::from_bundle(bundle)` は `bundle.runtime_parts()` から得られる `RuntimeParts<B::MailboxRuntime>` を展開し、必要な `ArcShared<R>` や設定値へ変換したうえで既存 `InternalActorSystem` を構築する責務を担う。
- `RuntimeParts<R>` には MailboxRuntime 本体、Scheduler ファクトリ、ReceiveTimeout ファクトリなどを格納し、旧 API が要求する依存をまとめて受け渡す。共通部品は既存実装（`modules/actor-*` 配下のユーティリティなど）を再利用し、新規コードは薄いラッパや構成要素の結合に限定する。`TestHarnessBundle` の実装では `MailboxHandleFactoryStub`、`SchedulerBuilder::<DynMessage, _>::priority()`、`NoopReceiveTimeoutSchedulerFactory` をそのまま活用している。
- `GuardianStrategy` など旧 API のジェネリクスは依然として `MailboxRuntime` を型引数に取るため、新 API 側では明示的に `B::MailboxRuntime` を利用する型境界を文書化・実装する。
- これにより `NewActorSystem` 本体は `B: NewActorRuntimeBundle` の抽象に集中でき、既存実装の型境界（`MailboxRuntime` など）を `new_runtime` 内部に閉じ込めることが可能になる。
- 将来的に `InternalActorSystem` 自体を刷新する場合も、`NewInternalActorSystem` の内部実装を差し替えるだけで新 API の表面を保てる。

## Cargo 構成と機能フラグ
- `core/Cargo.toml` に `new-runtime`（仮）フィーチャを追加し、新 API の公開タイミングを制御。
- 初期段階ではフィーチャ無効を既定とし、開発中は `cargo test -p nexus-actor-core --features new-runtime` を使用。
- 組み込み向けの差し替えを検証するため、`embedded` フィーチャと組み合わせたビルドを準備する。

## マイグレーションフェーズ
| フェーズ | 入力 | 作業 | 出力/完了条件 |
| --- | --- | --- | --- |
| A: スケルトン追加 | 既存 core クレート | `new_runtime.rs` と配下ファイルのひな形作成、trait/struct のスタブ定義 | 新APIが `cargo check -p nexus-actor-core --features new-runtime` でビルド成功 |
| B: Tokio 実装 | フェーズA成果、`protoactor-go` 参照 | `HostTokioBundleNew` と Mailbox/Scheduler 実装、互換アダプタの最初のバージョンを追加 | 既存サンプルをアダプタ経由で動作確認するテストを追加 |
| C: Embedded/Test 実装 | フェーズB成果、既存 embedded 設計メモ | `EmbeddedBundleNew` (Noop版) と `TestHarnessBundleNew` を整備、Feature フラグ連携 | `cargo check --target thumbv6m-none-eabi --features new-runtime` 成功 |
| D: ドキュメント/サンプル更新 | フェーズC成果 | README や設計メモを新 API へ更新、旧 API との差分説明を追記 | 主要ドキュメントが新旧併記状態になり、移行ガイドが完成 |

## テスト計画
- ホスト: `cargo test -p nexus-actor-core --features new-runtime`
- クロス: `cargo check -p nexus-actor-core --target thumbv6m-none-eabi --features new-runtime`
- Embedded 追加時に `cargo check -p nexus-actor-core --target thumbv8m.main-none-eabi --features new-runtime`
- 互換アダプタ用の統合テストを `core/src/new_runtime/tests/compat.rs`（ファイル名案）に配置し、旧 API から新 API を呼ぶケースをカバー。

## 実装時のレビュー所見（2025-10-12）
- Claude Code 初回実装では `Element` トレイトを `crate::api` からインポートしようとしてビルドに失敗した。実際の定義は `cellex_utils_core_rs::Element` なので、`use cellex_utils_core_rs::Element;` を基準とすること。
- `InternalActorSystem<M, R, Strat>` の第 2 型引数には `MailboxRuntime` 実装が要求されるため、そのまま `B: NewActorRuntimeBundle` を渡すと型が不一致になる。`NewActorRuntimeBundle` から **必ず** `MailboxRuntime` を取り出して `InternalActorSystem` に渡すブリッジ層（例: `runtime_parts()` が `ArcShared<R>` などを束ねた構造体を返す設計）を明文化する必要がある。
- `FailureEventHandler` / `FailureEventListener` は `crate::shared` 直下には存在せず、公開エイリアス経由でアクセスする。実装時は `use crate::{FailureEventHandler, FailureEventListener};` のように再エクスポートを利用する。
- `#![deny(clippy::todo)]` などのリンタ設定により `todo!()` や `unimplemented!()` は許容されない。最小実装でも `fn new(bundle: B) -> Self` を実装し、`NewActorRuntimeBundle` から取得した依存で `InternalActorSystem` を初期化するガイドを本メモに追加しておく。
- 日本語コメント規約を守るため、Rust コード内の通常コメントは日本語で記述する。英語コメントが必要な場合は rustdoc (`///`) として書くか、設計メモ内で日本語に言い換える。

## TODO / オープン課題
- [ ] 優先度:高 `NewActorRuntimeBundle` / `NewMailboxRuntime` の設計詳細を詰め、必須メソッドとライフタイム境界を決定する（依存: フェーズA）。
- [ ] 優先度:高 コア trait 名称（`ActorRuntimeBundle` vs `ActorRuntime` など）を確定し、ドキュメントとコードの命名を統一する（依存: フェーズA）。
- [ ] 優先度:高 `RuntimeParts` と `NewActorRuntimeBundle::runtime_parts()` の仕様を策定し、旧 API へ受け渡す構造と型制約を整理する（依存: フェーズA）。
- [ ] 優先度:中 互換アダプタで再利用する設定型の洗い出しと、既存 API とのマッピング表作成（依存: フェーズA）。
- [ ] 優先度:中 組み込みターゲット向けの `Noop` Mailbox/Scheduler 実装プロトタイプ作成（依存: フェーズA）。
- [ ] 優先度:低 新旧 API の共存期間を想定した deprecation ポリシー案の策定（依存: フェーズD）。

## 次のステップ
1. フェーズAのスケルトンを `new_runtime/` 配下に追加し、最小限の `NewActorRuntimeBundle` trait をコンパイル可能な状態にする。
2. 互換アダプタの要件を整理し、既存 `ActorRuntimeBundle` からの変換項目を列挙する。
3. Claude Code / Claude 4.5 にタスクを振る場合は、本メモと併せて対象フェーズ・想定コマンド・関連資料を提示する。
