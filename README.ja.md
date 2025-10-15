# cellex-rs

英語版は `README.md` を参照してください。日本語でも同等の情報を提供するため、本稿は次の構成でまとめています。

## 目次
- [概要](#概要)
- [クイックスタート](#クイックスタート)
- [主な機能](#主な機能)
- [アーキテクチャ概要](#アーキテクチャ概要)
- [開発フロー](#開発フロー)
- [名称とコンセプト](#名称とコンセプト)
- [進捗ステータス](#進捗ステータス)
- [参考資料](#参考資料)
- [ライセンス](#ライセンス)

## 概要

| テーマ | 内容 |
| --- | --- |
| 型付きビヘイビア | `Behavior<U, R>` DSL、`Context<'_, '_, U, R>`、`ActorRef<U, R>` による型安全なメッセージ処理 |
| ランタイム移植性 | `std` / `no_std + alloc` / Tokio / Embassy / RP2040・RP2350 クラス MCU に対応 |
| 監視とレジリエンス | ガーディアン階層、Restart/Resume/Stop 指示、エスカレーションシンク、Watch/Unwatch 通知 |
| スケジューリング | 優先度付きメールボックス、`dispatch_next` / `run_until` 等の非同期 API、組み込み向けブロッキングループ |
| エコシステム | `actor-core`・`actor-std`・`actor-embedded`・`remote`・`cluster`・`utils` など分割モジュール |

## クイックスタート

### 必要環境
- Rust 安定版（`rust-toolchain.toml` 参照）
- `cargo` / `rustup`
- 任意: ホスト向け `tokio`、組み込み向け `embassy-executor`

### 依存追加

```shell
cargo add cellex-actor-core-rs
# ホスト（Tokio）環境で利用する場合
cargo add cellex-actor-std-rs --features rt-multi-thread
```

### 最小サンプル（Tokio）

```rust
use cellex_actor_core_rs::{ActorSystem, Behaviors, Props};
use cellex_actor_std_rs::TokioMailboxRuntime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut system: ActorSystem<u32, _> = ActorSystem::new(TokioMailboxRuntime);
  let mut root = system.root_context();

  let props = Props::with_behavior(|| {
    Behaviors::receive(|_ctx, value: u32| {
      println!("受信: {value}");
      Ok(Behaviors::same())
    })
  });

  let actor = root.spawn(props)?;
  actor.tell(42)?;
  root.dispatch_next().await?; // キューを1件処理

  Ok(())
}
```

### 組み込み向けクロスチェック

```shell
# RP2040 (thumbv6m)
cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi
# RP2350 クラス (thumbv8m.main)
cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi
```

## 主な機能

- **型付き Actor DSL** — `Behavior`, `BehaviorDirective`, `Props` で純粋関数的にアクター挙動を記述。
- **優先度付きメールボックス** — システムメッセージとユーザーメッセージを同居させつつ制御系を先行処理。
- **監視階層** — ガーディアン戦略、Watch/Unwatch、Failure 情報のエスカレーションをサポート。
- **非同期スケジューリング** — `run_until` / `run_forever` / ブロッキングループでホスト・MCU を問わず常駐運転。
- **Shared 抽象** — `MapSystemShared`、`ReceiveTimeoutFactoryShared`、`ArcShared` などを通じて std / no_std の両方で共有資源を扱いやすく。
- **拡張性** — 拡張レジストリ、Failure Event ハブ、remote / cluster モジュールで将来の分散化に備える。

## アーキテクチャ概要

| パス | 役割 |
| --- | --- |
| `modules/actor-core` | 中核の typed ランタイム、ビヘイビア、スケジューラ、ガーディアン、メールボックス |
| `modules/actor-std` | Tokio 用ファクトリやランタイムドライバ |
| `modules/actor-embedded` | `no_std + alloc` アダプタ、Embassy 向けディスパッチャ、MCU サンプル |
| `modules/remote-core` / `remote-std` | gRPC ベースのリモート配送・エンドポイント監視 |
| `modules/cluster-core` | Gossip / sharding などクラスタリング基盤 |
| `modules/utils-*` | `ArcShared` など共通ユーティリティ |
| `docs/design` | 設計メモ（dispatch 移行、typed DSL、mailbox split 等） |
| `docs/worknotes` | 運用メモ・ハウツー（Tokio/Embassy ディスパッチャ、ロードマップ断片など） |

## 開発フロー

| 目的 | コマンド |
| --- | --- |
| フォーマット | `cargo +nightly fmt` または `makers fmt` |
| Lint | `cargo clippy --workspace --all-targets` |
| テスト（ホスト） | `cargo test --workspace` |
| カバレッジ | `cargo make coverage` または `./coverage.sh` |
| 組み込みクロスチェック | 上記 [組み込み向けクロスチェック](#組み込み向けクロスチェック) |

## 名称とコンセプト

- **語源:** `cellex = cell + ex`。`cell` は自律分散、`ex` は「外へ・超えて・交換する」を意味するラテン語由来の接頭辞。
- **三層の意味:**
  1. *Cell Exchange* — 細胞膜を越える物質交換のようなメッセージング。
  2. *Cell Execute* — 各細胞（アクター）が並行・自律的にふるまう。
  3. *Cell Exceed* — 単なる集合を超えて創発的な分散知性を実現。
- **発音:** `cel-lex`（セレックス）。親しみやすい「セル」と力強い「レックス (rex)」の響きを合わせ持つ。
- **プロジェクトメッセージ:**
  > 生命体の細胞のように、cellex の各アクターは独立して動作しながらシームレスに通信し、分散協調によって創発的な知性を生み出す。

## 進捗ステータス

- `QueueMailbox::recv` は `Result<M, QueueError<M>>` を返します。`Ok` 以外は閉鎖・切断シグナルなので停止処理を明示的に実装してください。
- `PriorityScheduler::dispatch_all` は非推奨です。`dispatch_next` / `run_until` / `run_forever` を利用してください（詳細は [dispatch 移行ガイド](docs/design/2025-10-07-dispatch-transition.md)）。
- Typed DSL は利用可能ですが、`map_system` をユーザー定義 enum へ拡張する課題や統合テスト強化が進行中です（[Typed DSL MUST ガイド](docs/worknotes/2025-10-08-typed-dsl-claude-must.md) を参照）。

## 参考資料

- [Typed Actor 設計メモ](docs/design/2025-10-07-typed-actor-plan.md): ビヘイビア／コンテキスト／SystemMessage 映射の設計方針。
- [Dispatcher Runtime ポリシー](docs/sources/nexus-actor-rs/docs/dispatcher_runtime_policy.md): 旧 `nexus` 世代のシャットダウン指針（概念は流用可能、API は要読み替え）。
- [ベンチマークダッシュボード](https://j5ik2o.github.io/cellex-rs/bench_dashboard.html): 週次ベンチの推移（`benchmarks/history/bench_history.csv`）。
- [ActorContext ロック計測レポート](docs/sources/nexus-actor-rs/docs/benchmarks/tracing_actor_context.md): ロック待ち分析（cellex API 名へ読み替え推奨）。
- [ReceiveTimeout DelayQueue PoC](docs/sources/nexus-actor-rs/docs/benchmarks/receive_timeout_delayqueue.md): DelayQueue を用いた receive timeout 実験。
- [Actor トレイト統一リリースノート](docs/sources/nexus-actor-rs/docs/releases/2025-09-26-actor-trait-unification.md): `BaseActor` 廃止と `ActorSpawnerExt` 導入の背景。
- [レガシーサンプル一覧](docs/sources/nexus-actor-rs/docs/legacy_examples.md): 旧実装サンプルの一覧。移行時の参照に利用可能。
- [Tokio ディスパッチャ手順](docs/worknotes/2025-10-07-tokio-dispatcher.md) / [Embassy ディスパッチャ手順](docs/worknotes/2025-10-07-embassy-dispatcher.md)。
- `modules/actor-embedded/examples/embassy_run_forever.rs`: Embassy executor との連携最小コード。

## ライセンス

本プロジェクトは MIT ライセンスおよび Apache-2.0 ライセンスのデュアルライセンスです。いずれかを選択して利用できます。
