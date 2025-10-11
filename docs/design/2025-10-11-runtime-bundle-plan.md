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

### フェーズ 2: Scheduler 抽象の切り出し（進行中）
- `Scheduler` トレイト（spawn_actor / dispatch_next / run_forever）を定義し、`PriorityScheduler` を実装として登録。
- `ActorRuntimeBundle` に `scheduler: Arc<dyn Scheduler>` を格納し、MailboxFactory とは独立に差し替えられるようにする。
- MailboxFactory 側は必要な最小限のインターフェース（Queue / Signal）へ整理し、Scheduler からの依存を縮小する。
- 進捗メモ:
  - `PriorityScheduler::spawn_actor` が `Box<ActorHandlerFn>` を受け取るようになり、今後トレイト化・オブジェクト化しやすい形に整備済み。（commit 7aea9d0 以降）
  - `ActorRuntimeBundle` が `cellex_utils_core_rs::sync::Shared` 系のハンドルで MailboxFactory / SchedulerBuilder を一貫保持するよう更新済み。DI ポリシーに沿って no_std / std 双方で同一 API を提供できる状態になった。
  - `ActorRuntimeBundle::with_scheduler_builder(_shared)` / `scheduler_builder` を公開 API 化。外部クレートでも Shared された `SchedulerBuilder` を注入できるようになり、Embedded 向けバンドル組立ての前提が整った。
  - `actor-embedded::embassy_scheduler_builder()` と `ActorRuntimeBundleEmbassyExt::with_embassy_scheduler()` を追加し、Embassy executor 上でも Shared ハンドル経由でスケジューラを差し替えられる最小実装を用意。

### フェーズ 3: 追加コンポーネントの統合
- ReceiveTimeout ドライバ、Escalation/Event リスナー、FailureHub などをバンドル内に移管。
- Host（std）、Embedded（no_std + alloc）、Remote 専用バンドルをそれぞれ定義し、必要なコンポーネントを組み合わせる。
- `ActorSystemBuilder` を導入し、アプリケーション側が個別コンポーネントを上書きできる設定 API を提供する。

## マイルストーン / TODO
- [x] フェーズ 1 実装: `ActorRuntimeBundle` 追加、既存 API の 移行。（commit 7aea9d0, 843072e）
- [ ] フェーズ 2 設計レビュー: Scheduler トレイト定義と既存テストの影響調査。
- [ ] フェーズ 3 要件整理: Timeout・EventListener 等の利用箇所棚卸し。
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
