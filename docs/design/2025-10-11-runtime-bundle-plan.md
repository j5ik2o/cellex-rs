# Actor Runtime 抽象リファクタリング計画 (2025-10-11)

## 現状の課題
- `ActorSystem::new` は `R: MailboxFactory` だけを受け取っており、Scheduler や ReceiveTimeout ドライバなど実行基盤の差し替えが想定されていない。
- `PriorityScheduler<R>` が `MailboxFactory` に強く依存しているため、Mailbox と Scheduler が実質的に結合している。
- Embedded / Tokio / Remote など異なるプラットフォーム向けに必要なコンポーネント（Scheduler、Timeout、EventListener、Metrics 等）をまとめて提供する仕組みが存在しない。

## ゴール
1. Mailbox と Scheduler を疎結合にし、プラットフォームごとに任意の組み合わせを選べるようにする。
2. `ActorSystem` へ渡すパラメータを「実行基盤バンドル (ActorRuntime)」として整理する。
3. `ReceiveTimeout` ドライバやイベント通知、メトリクスなど追加コンポーネントを段階的にバンドルへ移せるようにする。

## フェーズ別計画

### フェーズ 1: ランタイムバンドルの導入 ✅
- `ActorSystem::new(runtime: ActorRuntimeBundle)` と同等の構造体を導入済み。
- 現状は `mailbox_factory` のみ保持し、互換 API (`ActorSystem::new(mailbox_factory)`) と併用可能。
- コード位置: `modules/actor-core/src/api/actor/system.rs`

### フェーズ 2: Scheduler 抽象の切り出し ✅
- `Scheduler` トレイト（spawn_actor / dispatch_next / run_forever）を定義し、`PriorityScheduler` を実装として登録。`SchedulerBuilder` を公開し、プラットフォーム側でラッパを組み立てられる状態にした。
- `ActorRuntimeBundle` は `SchedulerBuilder` を Shared で保持。Tokio/Embassy など環境依存の実装は `actor-std` / `actor-embedded` 側で拡張トレイト（`with_tokio_scheduler` / `with_embassy_scheduler`）として提供。
- MailboxFactory → MailboxBuilder/Handle 分解の骨子（PriorityMailboxSpawnerHandle 導入、RuntimeBundle からの共有ハンドル提供、SchedulerSpawnContext の再設計）を実装済み。Scheduler は `SchedulerSpawnContext` を介して mailbox handle を受け取り、Factory 依存を解消した。

#### フェーズ2の成果まとめ
- Scheduler と Mailbox 生成経路の依存を切り離すためのインターフェース方針を確定。
- RuntimeBundle で MailboxFactory を共有ハンドル化する要件定義を完了。
- Tokio / Embassy ラッパーが SchedulerBuilder 経由で差し替え可能であることを検証済み。

### フェーズ 3: 追加コンポーネントの統合（着手）

#### 3-1 ReceiveTimeoutDriver 抽象化
- `ReceiveTimeoutSchedulerFactory` を RuntimeBundle に登録し、Tokio/Embassy 向け実装をモジュール側に分離。
- MailboxSpawner と連携する `SchedulerSpawnContext` 拡張を実装し、Scheduler がタイマーファクトリに直接アクセスしない構成を整える。
- `ActorSystemConfig::with_receive_timeout_factory` 互換を維持しつつ、Bundle/API 双方からドライバを設定可能にする。

#### 3-2 EventListener / EscalationHandler のバンドル統合
- RuntimeBundle に Root EventListener / EscalationHandler を保持するフィールドと組み込み API を追加。
- `InternalActorSystemSettings` へ統合し、Scheduler 初期化時にリスナーを注入する。
- プラットフォーム別バンドル（std / embedded / remote）が独自ハンドラを簡潔に配線できる DSL を整備。

#### 3-3 MetricsSink / 拡張コンポーネント
- メトリクス送信口を抽象化する `MetricsSink` トレイトを定義し、Bundle から Scheduler / ActorSystem に注入する導線を追加。
- 初期スコープ: `Noop`, `Prometheus`, `defmt`（embedded）実装。Remote バンドルでは gRPC 連携用の Hook を予定。

#### 3-4 バンドル プロファイル定義
- Host( std ) / Embedded ( no_std + alloc ) / Remote (std + gRPC) の 3 プロファイルを定義し、
  - 既定 Scheduler / Mailbox / TimeoutDriver / Metrics / EventListener / EscalationHandler / FailureHub を一覧化。
  - `ActorRuntimeBundle::host()` / `::embedded()` / `::remote()` のコンビニエンス関数で組み立てる。
- プロファイル生成を補助する `ActorRuntimeBundleBuilder` を導入し、任意コンポーネント差し替えを可能にする。

#### 3-5 ActorSystemBuilder の整備
- RuntimeBundle と ActorSystemConfig を統合する `ActorSystemBuilder` を新設。
- アプリケーションが Builder パターンで RuntimeBundle の個別コンポーネントを上書きできる API を提供。
- 組み込みバンドルを起点にドライバ・イベント・メトリクスを差し替えるサンプルを docs/worknotes に追加。

#### 3-6 検証計画
- ユニットテスト: バンドル別に ReceiveTimeout / EventListener / MetricsSink が注入されることを確認するテストを `actor-core` に追加。
- 統合テスト: `actor-std` / `actor-embedded` で同一シナリオを実行し、バンドル差し替え時の動作を検証。
- ドキュメント: README / ワークノート / API Docs に新バンドル API の使用例を追記。

## マイルストーン / TODO
- [x] フェーズ 1 実装: `ActorRuntimeBundle` 追加、既存 API の 移行。（commit 7aea9d0, 843072e）
- [x] フェーズ 2 設計レビュー: Scheduler 抽象刷新と MailboxFactory 分離案の確定。
- [ ] フェーズ 3 実装: ReceiveTimeout / Event / Metrics 統合、および Bundle Builder の提供。
- [ ] ドキュメント更新: README / ワークノートに新しい実行モデルのガイドを追記。

## 参考リンク
- `modules/actor-core/src/runtime`（Scheduler 実装）
- `modules/actor-embedded/src/embassy_dispatcher.rs`
- `docs/worknotes/2025-10-07-embassy-dispatcher.md`

## Scheduler 抽象詳細
- **インターフェース案**
  - `trait Scheduler`: `spawn`, `tick`, `notify_ready`, `shutdown` を定義。protoactor-go の `Scheduler` を参考にし、タスク駆動 + コールバック登録型。
  - `SchedulerContext`: Mailbox とは無関係に Actor の ID・優先度・工場関数を受け渡す軽量 DTO を想定。
- **PriorityScheduler のリファクタリング方針**
- 既存の `PriorityScheduler` は `MailboxFactory` に直接アクセスしているため、`MailboxBuilder`（Factory 的責務）と `MailboxHandle`（Scheduler から利用する操作）に分割する。
  - 優先度キュー (`binaryheap`) と `tokio::task::JoinHandle` の管理は Scheduler 内に閉じ込める。
  - `#[cfg(feature = "embedded")]` では `heapless::binary_heap` + `embassy_executor::Spawner` ベースの実装を用意する。
- **バックプレッシャーと計測の差し込みポイント**
  - メールボックス溢れを Scheduler で感知できるように `spawn` 戻り値へ `Result<MailboxHandle, SpawnError>` を導入。
  - `tick` / `notify_ready` の境界で `tracing::instrument` を使い、後続でメトリクス収集する。

## ActorRuntimeBundle API 仕様案
- **構造体レイアウト**
  - `pub struct ActorRuntimeBundle { pub mailbox: Shared<dyn MailboxBuilder>, pub scheduler: Shared<dyn Scheduler>, pub timeout_driver: Shared<dyn ReceiveTimeoutDriver>, pub metrics: Shared<dyn MetricsSink>, ... }`
  - 依存性注入パターンを明示するため `cellex_utils_core_rs::sync::Shared` を統一使用。no_std 向けは `ArcShared` / `RcShared` などバックエンド差し替えで対応。
- **ビルダー API**
  - `ActorRuntimeBundle::builder()` でデフォルト構成（Tokio + PriorityScheduler + DefaultMailbox）を生成し、各項目を `.with_scheduler(_)` 等で差し替え。
  - Embedded バンドルは `ActorRuntimeBundle::embedded()`、Remote バンドルは `ActorRuntimeBundle::remote()` といったコンビニエンス関数を提供。
- **ActorSystem 連携**
  - `ActorSystem::builder(runtime_bundle)` -> `.with_name("app")` -> `.build()` のフローへ移行。
  - 旧 API (`ActorSystem::new(mailbox_factory)`) は `ActorRuntimeBundle::from_mailbox(mailbox_factory)` を内部で呼び出す互換層。

## コンポーネント移行ロードマップ
- **ReceiveTimeoutDriver**
  - 現在 `core::runtime::receive_timeout` で tokio タイマーに依存。`trait ReceiveTimeoutDriver` を導入し、Host/Embedded で実装を切り替え。
  - Timeout のキャンセルタイミングを明示するため、`ActorRef::cancel_receive_timeout` を Driver へ委譲。
- **EventListener / FailureHub**
  - `EventStream`/`FailureHub` を `ActorRuntimeBundle` 内で初期化し、System が起動時に subscribe する。
  - 監視対象イベント（成功/失敗/停止）を `EventFilter` で定義し、リモート専用フックを簡単に差し込めるようにする。
- **MetricsSink**
  - `trait MetricsSink { fn record(&self, metric: MetricEvent); }` を定義し、Prometheus / defmt ロガー / No-op 実装をバンドルごとに登録。

## テスト戦略
- **ユニットテスト**: `ActorRuntimeBundle` ビルダーの差し替え確認、Scheduler が Mailbox 依存を持たないことのモックテスト。
- **統合テスト**: `core/tests.rs` に Host バンドル + Embedded バンドル双方で同一シナリオが動くクロスプラットフォームテストを追加。
- **プロパティテスト**: メールボックスサイズとスケジューラ tick 頻度の関係を `proptest` で検証し、デッドロックを検知。
- **クロスビルド検証**: `cargo check --target thumbv6m-none-eabi` / `thumbv8m.main-none-eabi` を CI に組み込み、`cfg` 条件の漏れを防ぐ。

## リスクと対応策
- **抽象化肥大化**: バンドルにコンポーネントを詰め込み過ぎると理解しづらい → `ActorRuntimeBundleParts` を導入し、必要なモジュールだけを組み立てる分割 API を検討。
- **性能劣化**: Scheduler を dyn dispatch にするとホットパスで遅くなる懸念 → ベンチマーク (`criterion`) で現行実装比を計測し、必要なら `enum_dispatch` 等による静的ディスパッチ化を検討。
- **Embedded 対応の複雑化**: no_std で `Arc` が使用不可な環境 → feature flag で `Rc` または `&'static` 提供を検討し、`alloc` 依存を明示。
- **レガシー API 置換の漏れ**: 段階的移行中の破壊的変更でテストが不十分 → `grep "ActorSystem::new("` で使用箇所を洗い出し、PR チェックリストに追加。

## 今後の検討事項
- `ActorSystemBuilder` で `SystemMetrics` 等の非同期初期化を同期化する仕組み（`async fn build()` の是非）。
- Mailbox 監視 API (`MailboxProbe`) をバンドル経由で差し込めるようにし、テスト用途のフックを整備。
- protoactor-go の `ProcessRegistry` との整合性確認。Rust 側では `Arc<Registry>` を共有するが、Embedded では静的テーブル化も検討。
- Remote バンドルと gRPC Transport の依存順序（Channel 初期化と System 起動順）をどこで調停するか。

## 直近アクションプラン
1. Scheduler トレイトドラフトを PR にまとめ、`PriorityScheduler` を一時的に adapter 経由で接続。
2. MailboxFactory の再分割（Builder / Handle）を実施し、Scheduler からの依存を解消。
3. ReceiveTimeoutDriver の抽象化と tokio 実装を同 PR に含め、テストを `#[cfg(feature = "tokio" )]` で分離。
4. Embedded バンドルの PoC を `modules/actor-embedded` に追加し、`cargo check --target thumbv6m-none-eabi` を通す。

## 用語整理
- **ActorRuntimeBundle**: ActorSystem を起動するためのコンポーネント集合。環境ごとに差し替え可能。
- **Scheduler**: Mailbox からメッセージを取り出し、Actor を評価する駆動ループ。
- **MailboxBuilder / MailboxHandle**: メールボックス生成と操作の責務を分離した抽象。
- **ReceiveTimeoutDriver**: アクターの ReceiveTimeout を管理するタイマードライバ。
- **MetricsSink**: System 全体のメトリクスイベント集約ポイント。
