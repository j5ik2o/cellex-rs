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

### MailboxFactory 再分割 詳細作業計画

#### ゴールと成果物
- MailboxFactory を `MailboxBuilder` と `MailboxHandle`（仮称）に明確分離し、Scheduler 側が Builder 実装へ直接依存しない構造を確立する。
- `ActorRuntimeBundle` が Mailbox 生成ハンドルを共有資源として保持し、Scheduler 初期化時は `SchedulerSpawnContext` 経由でハンドルを受け取るフローを完成させる。
- 既存ユニットテスト群（`modules/actor-core/src/api/actor/tests.rs` 等）を全てパスさせ、クロスビルド (`thumbv6m-none-eabi` / `thumbv8m.main-none-eabi`) を阻害しないことを確認する。

#### 事前準備 (0.5 日)
1. `protoactor-go/actor/mailbox` 実装の `producer` / `invoker` 分離例を再読し、Rust 化する際の責務境界を整理する。
2. 現行 `MailboxFactory` の利用箇所を `rg "MailboxFactory" -g"*.rs" modules/actor-core` で洗い出し、Builder/Handle それぞれに置き換える必要がある API を一覧化する。
3. 旧実装（`docs/sources/cellex-rs-old/`）の Mailbox 関連を確認し、再利用可能なテストケースやベンチマークがあればメモする。 → AGENTS.mdが間違っていました。再度AGENTS.mdを確認して。

#### 実装ステップ (1.5 日)
1. 型設計
   - `modules/actor-core/src/runtime/mailbox/` に `mailbox_builder.rs`（仮）と `mailbox_handle.rs` を追加し、Builder/Handle のトレイト定義と最小限のデフォルト実装を用意。
   - `MailboxFactory` は暫定で Builder/Handle 両方をカプセル化する façade として残し、既存呼び出しへの移行期間を確保。
2. RuntimeBundle 拡張
   - `ActorRuntimeBundle` に `mailbox_handle: Arc<dyn MailboxHandle>` フィールドを追加し、Builder 注入パスとは独立に Handle を Scheduler へ配布できるようにする。
   - `ActorRuntimeBundleBuilder`（未実装の場合は仮組み）に Builder/Handle 両方の setter を追加し、Tokio / Embedded プロファイルでのデフォルト値を定義。
3. Scheduler 連携
   - `PriorityScheduler` 内の `MailboxFactory` 直接参照を `SchedulerSpawnContext::mailbox_handle()` へ差し替える。
   - `SchedulerSpawnContext` に Builder ではなく Handle を注入するためのコンストラクタ／ Getter を追加し、コンテキスト生成箇所を更新。
4. Mailbox 実装更新
   - 標準 Mailbox (`default_mailbox.rs` など) を Builder/Handle に準拠するよう改修し、`PriorityMailboxSpawnerHandle` 等の命名や共有方法を見直す。
   - 必要に応じて `Arc<dyn MailboxInvoker>` など補助トレイトを導入し、Handle が Scheduler スレッドセーフ性を保証できるよう調整。
5. 互換レイヤー整備
   - 既存 `ActorSystem::new(mailbox_factory)` 呼び出しを非推奨にし、新 API への誘導をコメントとドキュメントで明示。
   - 互換期間中は `MailboxFactory` が内部で Builder/Handle 両方を生成・返却する暫定実装を提供し、段階的に呼び出し側を差し替える。

#### テスト・検証 (0.5 日)
- `cargo test -p nexus-actor-core-rs` を実行し、Mailbox 差し替えテスト（特にスケジューラとの統合テスト）を追加して成功を確認。
- `cargo check -p nexus-actor-core-rs --target thumbv6m-none-eabi` および `--target thumbv8m.main-none-eabi` を実行し、no_std 対応が壊れていないことを保証。
- MailboxFactory 移行に伴う API 変更点を `CHANGELOG.md` または設計ドキュメントに追記し、後続フェーズの依存チームへ共有。

#### 品質ゲートとレビュー
- Pull Request では以下を必須エビデンスとして添付：
  - 実行コマンドログ（`cargo test` / `cargo check --target ...`）。
  - 新旧構造のクラス図またはシーケンス図を docs/worknotes/ に配置し、PR から参照。
  - Scheduler 側で MailboxFactory への直接依存がゼロになったことを示す `rg` 結果の抜粋。
- コードレビューではコンカレンシー安全性（`Send` / `Sync` 境界）とハンドルのライフタイム設計を重点確認ポイントに設定。

#### リスクと緩和策
- **API 破壊の波及**: Builder/Handle 追加で呼び出し側修正が広範囲に及ぶ → 互換レイヤーを段階的に残し、モジュール別に PR を分割。
- **no_std 対応の破綻**: `Arc` 依存や `tokio` 特有型が紛れ込むリスク → `cfg(feature = "std")` ガードと `alloc` ベースの抽象に限定するコードレビュー項目を追加。
- **パフォーマンス劣化**: Handle 経由呼び出しで余計な `Arc` クローンが発生 → Criterion ベンチを `modules/actor-core/benches` に追加し、メッセージ吞吐の回帰比較を行う。

#### 2025-10-12 実装ログ（進捗）
- `MailboxHandleFactoryStub<R>` を公開構造体として定義し、`from_runtime`/`priority_spawner` を通じてランタイムに依存した MailboxHandle を生成できるようにした（`modules/actor-core/src/api/actor/system.rs:104`）。
- `SchedulerSpawnContext` は `mailbox_spawner` の代わりに `MailboxHandleFactoryStub` を受け取り、Scheduler 側で必要なタイミングにハンドルを派生させる構造へ移行（`modules/actor-core/src/runtime/scheduler/actor_scheduler.rs:29`）。
- `PriorityScheduler` / `InternalRootContext` / 各テストを新しいコンテキスト構造に合わせて更新し、`MailboxFactory` 直接依存を段階的に縮小（例: `modules/actor-core/src/runtime/system/internal_root_context.rs:49`、`modules/actor-core/src/runtime/scheduler/priority_scheduler.rs:105`）。
- `ActorRuntimeBundle::priority_mailbox_spawner` は束縛中のランタイムクローンから stub を作成する実装へ変更し、外部呼び出しでも統一的に MailboxHandle を取得可能にした。
- RuntimeBundle / ActorSystemConfig に `MetricsSinkShared` を追加し、スケジューラ初期化時に `set_metrics_sink` で注入されるパスを整備。Tokio / Embassy ラッパーおよび `PriorityScheduler`／`ImmediateScheduler` にハンドラを実装し、設定値の優先順位（Config > Bundle）をユニットテスト化した。
- `PriorityScheduler` 内でアクター登録／停止およびメッセージの enqueue/dequeue 時に `MetricsEvent` を発行し、テストで `MailboxEnqueued` / `MailboxDequeued` の対が届くことを検証した。
- RuntimeBundle 拡張で Tokio/Embassy 向け `ReceiveTimeoutSchedulerFactory` を自動登録（Tokio: `TokioReceiveTimeoutSchedulerFactory`, Embedded: `NoopReceiveTimeoutSchedulerFactory`）し、`Noop` ドライバを追加して全環境で ReceiveTimeout 依存が解決されるようにした。

#### 参考ソース確認メモ
- 2025-10-11 時点でリポジトリ内に `docs/sources/cellex-rs-old/` ディレクトリは存在しない。`find docs -maxdepth 4 -name "*cellex*"` や `rg "cellex-rs-old" -n` を実行したが、参照のみで実体は未配置。→ 旧実装は `docs/sources/nexus-actor-rs/`でした。
- 旧実装を参照する必要がある場合は、アーカイブ取得手段（過去リポジトリや別ブランチ、外部ストレージ）を確認するタスクを別途起票する。
- 当面は `docs/sources/nexus-actor-rs/` および protoactor-go の実装を一次資料として用いる。

### ActorRuntimeBundle MailboxHandle 配線設計案

#### 目的
- Scheduler と Mailbox の疎結合化を進める上で、RuntimeBundle から Scheduler へ MailboxHandle を安全かつ明示的に供給する経路を定義する。
- Host / Embedded / Remote など異なるプロファイルで共通の抽象を使い回し、将来的な Builder 差し替えにも耐える構造を用意する。

#### コンポーネント構成案
- `MailboxBuilder`: 各アクターの Mailbox を生成する責務を持つトレイト。`fn build(&self, spec: &SpawnSpec) -> MailboxPair` を想定。
- `MailboxHandle`: 生成済み Mailbox に対して enqueue / metrics / diagnostics を提供する操作インターフェース。Scheduler 側は Handle を通じてのみメッセージ操作を行う。
- `MailboxRegistry`: `Arc<dyn MailboxHandleFactory>` のように、Builder から Handle を生成して共有する補助構造。RuntimeBundle 内部で保持。
- `SchedulerSpawnContext`: Scheduler 起動時に RuntimeBundle から引き渡される初期化コンテキスト。`fn mailbox_handle(&self) -> Arc<dyn MailboxHandle>` を公開。
- `ActorRuntimeBundleCore`: RuntimeBundle の内部構造体。`mailbox_builder`, `mailbox_handle_factory`, `scheduler_builder` などをフィールドとしてまとめ、フィーチャごとに組み替える。

#### シーケンス概要
1. アプリケーションが `ActorRuntimeBundle::host()` 等でデフォルトバンドルを生成し、必要に応じて `.with_mailbox_builder(...)` `.with_mailbox_handle_factory(...)` で差し替え。
2. `ActorSystemBuilder::build()` が RuntimeBundle から `SchedulerBuilder` を取得し、`SchedulerBuilder::build(context)` を呼び出す。
3. `SchedulerSpawnContext` 生成時に `mailbox_handle_factory.provision()` を呼び、Scheduler へ `Arc<dyn MailboxHandle>` を渡す。
4. Scheduler は Handle 経由で Mailbox に enqueue し、Builder との直接結合は発生しない。
5. 新しい Mailbox が必要になった場合は Scheduler から Handle 経由で要求を発行し、Handle が内部で Builder を呼び出して `MailboxPair` を作成する（必要なら lazy-init）。

#### API ドラフト
```rust
pub trait MailboxHandle: Send + Sync {
  fn enqueue_user(&self, msg: PriorityEnvelope);
  fn enqueue_system(&self, msg: SystemMessage);
  fn stats(&self) -> MailboxStats;
}

pub trait MailboxHandleFactory: Send + Sync {
  fn provision(&self) -> Arc<dyn MailboxHandle>;
}

pub struct ActorRuntimeBundle {
  mailbox_builder: Arc<dyn MailboxBuilder>,
  mailbox_handle_factory: Arc<dyn MailboxHandleFactory>,
  scheduler_builder: Arc<dyn SchedulerBuilder>,
  // ...
}

impl SchedulerSpawnContext {
  pub fn mailbox_handle(&self) -> Arc<dyn MailboxHandle> { /* ... */ }
}
```

#### 追加検討事項
- no_std 環境では `Arc` が使えないため、`Shared<dyn MailboxHandle>` のような抽象ジャケットを導入し、バックエンドを `Arc` / `Rc` / `StaticRef` で切り替えられるようにする。
- `MailboxHandle` がトレース計測を担うかは検討が必要。`MailboxMetricsHook` のようなプラガブルなフックを Handle へ注入する案を比較する。
- Handle のライフサイクル管理（shutdown 時の drain / cancel）をどう扱うかを明文化する。必要であれば `MailboxHandle::shutdown()` を追加。
- Scheduler から Builder へのエラーパスは Handle が吸収し、`MailboxProvisionError` として上位へ伝播させる。エラー種別とリトライ戦略をドキュメント化する。

#### 次アクション
- `ActorRuntimeBundleCore` の型定義ドラフトを `modules/actor-core/src/api/actor/system.rs` に追加し、テスト用のスタブ実装を用意する。
- `SchedulerSpawnContext` の生成箇所（`runtime/scheduler/actor_scheduler.rs` など）を洗い出し、Handle 導線を差し込むための TODO コメントを設定する。
- no_std 用の `Shared` 抽象を `utils` クレートから再利用できるかを調査し、必要に応じて共通トレイトを拡張する。

## 用語整理
- **ActorRuntimeBundle**: ActorSystem を起動するためのコンポーネント集合。環境ごとに差し替え可能。
- **Scheduler**: Mailbox からメッセージを取り出し、Actor を評価する駆動ループ。
- **MailboxBuilder / MailboxHandle**: メールボックス生成と操作の責務を分離した抽象。
- **ReceiveTimeoutDriver**: アクターの ReceiveTimeout を管理するタイマードライバ。
- **MetricsSink**: System 全体のメトリクスイベント集約ポイント。
