# RootEscalationSink テレメトリ抽象化計画

最終更新: 2025-10-14（telemetry 基盤の最小実装を追加済み）

## 背景
- `modules/actor-core/src/api/supervision/escalation.rs:124` で `tracing::error!` を直接呼び出しており、`#[cfg(feature = "std")]` に依存している。
- リポジトリ方針としてランタイム本体に `std` フラグ分岐を持ち込まないこと、`no_std` 対応を阻害しないことが求められている。
- 既に `event_handler` / `event_listener` という外部通知フックが存在するが、ロギング責務の分離が不十分で、`no_std` でのログ出力手段が確立していない。

## ゴール
1. ランタイム本体から `tracing` 依存と `#[cfg(feature = "std")]` 分岐を除去する。
2. 失敗通知の拡張ポイントを整理し、`std` / `no_std` 双方で運用可能なロギング・モニタリング手段を注入できるようにする。
3. 既存の `event_handler` / `event_listener` の活用方針を明確化し、必要であれば API 調整を行う。

## 現状 / アプローチ概要
- `modules/actor-core/src/api/supervision/telemetry.rs` を新設し、`FailureTelemetry` トレイトおよび `NoopFailureTelemetry` / `TracingFailureTelemetry` を定義。
- `RootEscalationSink` は telemetry フィールドを保持し、`default_failure_telemetry()` によりビルド構成に応じた実装（`std,unwind-supervision` なら tracing、それ以外は null）を利用する。`tracing::error!` の直書きは解消済み。
- telemetry を外部から注入するための setter を公開済み。ただし `ActorSystem`/`ReadyQueueScheduler` など高レベル API ではまだ利用手段を整備していない。
- 今後は初期化時のデフォルト注入や、アプリケーション側で telemetry を差し替えるビルダー API の追加を検討する。

- `FailureTelemetry`（仮称）トレイトを導入し、`FailureInfo` を受け取って副作用を生じさせるインタフェースを定義する。
- `RootEscalationSink` は `FailureTelemetry` を保持し、イベント通知後に呼び出すのみとする。
- `std` 環境では `TracingFailureTelemetry` 実装を提供し、既存の `tracing::error!` ログを移植する。
- `no_std` 環境では `NoopFailureTelemetry` または `RingBufferFailureTelemetry`（将来の拡張）などの実装を注入できるようにする。
- ランタイム初期化時に telemetry 実装を注入する API を追加し、既存の `event_handler` と共存させる設計を検討する。

## 実施フェーズ
### フェーズ1: インタフェース整備（完了）
- `FailureTelemetry` トレイトおよび `NullFailureTelemetry` / `TracingFailureTelemetry` を追加。
- `RootEscalationSink` が telemetry を保持し、`tracing` 直接呼び出しを削除済み。

### フェーズ2: `std` 向け実装と注入経路（進行中 → 主要機能完了）
- `FailureSnapshot` / `FailureTelemetry` リファクタを適用し、`RootEscalationSink` がスナップショット経由で telemetry を呼び出すようにした。
- `ActorSystemConfig::with_failure_telemetry` を追加し、`InternalActorSystemSettings` でデフォルト telemetry（`default_failure_telemetry()`）を自動注入する経路を整備済み。
- 残タスクは `RootEscalationSink` のビルダー API 提供可否、および `NoopFailureTelemetry` の共有最適化。
- Telemetry 拡張の将来案は `docs/rfc/2025-10-root-escalation-telemetry.md`（Draft）に整理済み。
- `ActorSystemConfig::with_failure_telemetry_builder` を追加し、`TelemetryContext` を経由した初期化フックを提供。

### フェーズ3: `no_std` 運用確認（進行中）
- `NoopFailureTelemetry` での運用確認は必要（`thumb` チェックは `./scripts/ci-check.sh all` に任せる）。
- `defmt` 等への拡張案を別途検討する。
- `docs/design/D18-telemetry-defmt-next-actions.md` にビルダー API を用いた `defmt` 連携メモを追加し、実装方針を整理。

### フェーズ4: ドキュメントとテスト（未着手）
- `docs/design` に最終設計メモを追記、本ファイルを更新。
- ユニットテスト／統合テストで `FailureTelemetry` の呼び出し順・副作用を検証。
- `./scripts/ci-check.sh all` を実行し、CI チェックに備える。

## オープン課題
- Telemetry の注入ポイントをどこに置くか（`ActorSystemBuilder` 相当の API があるか要確認）。
- `event_handler` / `event_listener` と telemetry の責務分割（重複を避けるための再設計が必要か）。
- `no_std` 環境でのログ出力先（UART, RTT, defmt 等）を採用するか、利用者に委ねるか。
- `FailureInfo` の clone コスト最適化（大量発生時のパフォーマンス影響評価）。
- Telemetry 観測フックの有効化条件とオーバーヘッド評価（タイミング計測の閾値、MetricsEvent 粒度の検討）。

## 次アクション
1. `thumb` ターゲットでの `cargo check` を継続実行できるよう CI 定義を更新する（期限: 2025-10-21）。
2. `FailureTelemetryShared` ベンチ結果をカバレッジレポート／設計ドキュメントに取り込み、性能指標として共有する（期限: 2025-10-22）。
3. `FailureSnapshot` のフィールド拡張（メタタグ等）の RFC を草案化する（期限: 2025-10-24）。

### CI 取り込み計画（案）
- GitHub Actions に `ci-thumb-check.yml`（仮称）を追加し、`matrix` で `thumbv6m-none-eabi` / `thumbv8m.main-none-eabi` を同時実行。
- 依存ツール:
  - `rustup target add thumbv6m-none-eabi thumbv8m.main-none-eabi`
  - `cargo install cargo-binutils`（必要時）
- 実行コマンド: `cargo check -p cellex-actor-core-rs --target <thumb-target>`
- `workflow_call` 化して `./scripts/ci-check.sh all` からも呼び出せるように検討。
- 成果物: 成功ログを Artefact に保存し、fail 時はダンプを添付（`RUST_LOG=debug` 推奨）。

## 詳細設計メモ
### `FailureSnapshot` が必要な理由
- スーパーバイザ内部の mutable な状態をそのまま渡すと、telemetry 実装が意図せず再起動制御に干渉する恐れがあるため、読み取り専用スナップショットとして外部公開する。
- `FailureInfo` をそのまま再利用すると `std` 依存の文字列所有権やエラー型が混入しており、`no_std` で扱いづらい。`FailureSnapshot` で最小限のデータに再編成し、`alloc` 不要な構造を定義する。
- Telemetry 以外（例: event_listener, メトリクス収集）のコンポーネントも同じ失敗情報を共有できる共通フォーマットを用意し、API 互換性を保ちながら拡張しやすくする。
- 将来的にリングバッファや遠隔転送へ蓄積する際、シリアライズ／ストレージ向けの厳密なデータ境界が求められるため、変換コストの低いスナップショット型が必要。

### 命名方針メモ
- Rust には `null` が存在しないため、動作を行わない実装は `NoopFailureTelemetry` と命名して意味を明確化する。
- `NoopFailureTelemetry` は値オブジェクトとして常に存在させ、`Option` や `Result` でのラップを避けて呼び出し側の分岐を排除する。
- 今後追加する実装も、効果や記憶方式を直接示す名称（例: `RingBufferFailureTelemetry`）で統一し、利用者が役割を即座に理解できるようにする。

### `FailureTelemetry` トレイト仕様メモ
- `FailureTelemetry: SharedBound` を実装し、`target_has_atomic = "ptr"` の有無で `Send + Sync` を切り替える。
- 呼び出しシグネチャは `fn on_failure(&self, snapshot: &FailureSnapshot)` とし、副作用は `()` 戻り値で完結させる。
- ログ／メトリクス実装側で失敗を握り潰す運用とし、呼び出し元では結果を扱わない。

### `FailureSnapshot` 実装詳細
- `actor`: `ActorId`（Copy）を保持し、`actor()` getter で参照させる。
- `path`: `ActorPath` を clone して保持し、`segments()` 参照をそのまま利用できるようにした。
- `failure`: `ActorFailure` を clone で保持し、telemetry 側で `behavior()` などへアクセス可能にする。
- `metadata`: `FailureMetadata` を clone して保持。タグやトランスポート情報はそのまま再利用。
- `stage`: `EscalationStage` を保持し、ログ時に hop 数を確認できる。
- `description`: `String` に eagerly 変換し、`Cow` のライフタイム制約を取り除いた。
- `tags`: 最大 `MAX_FAILURE_SNAPSHOT_TAGS` 個の `TelemetryTag` を保持し、`component` / `endpoint` / `transport` と任意タグを順序付きで展開。

### ActorSystemConfig 連携メモ
- `ActorSystemConfig::with_failure_telemetry` を追加し、アプリケーションが独自 telemetry を `FailureTelemetryShared` 経由で注入できるようにした。
- `InternalActorSystemSettings` は config からのハンドルを優先し、設定が無い場合は `default_failure_telemetry()` を利用する。
- `ReadyQueueScheduler` / `CompositeEscalationSink` まで `FailureTelemetryShared` を受け渡し、Root で一度だけ clone する構成。

### event_handler / event_listener との責務整理
- `event_handler` は引き続き外部通知の汎用フックとして扱い、`FailureTelemetry` から重複して呼ばないようにする。
- `event_listener` は `FailureTelemetry` の観測結果を二次加工する場として再活用可能。必要に応じて `FailureEvent` を `event_listener` に転送するアダプタを提供する。
- 双方とも `no_std` 互換を維持するため、`alloc` を利用する処理は利用側に委ねる。

### 運用シナリオ例
- `std` + tracing: `TracingFailureTelemetry` をデフォルト注入し、各失敗イベントを `tracing::event!(Level::ERROR, ...)` で出力。
- `no_std` + defmt: 別クレートで `DefmtFailureTelemetry` を提供し、組み込み向けに `defmt::error!` を呼ぶ。
- 高信頼要求: `RingBufferFailureTelemetry` を用意して一定件数を保持し、`event_listener` 経由でダンプ。

## テスト戦略
- `modules/actor-core/src/api/supervision/tests.rs` に `test_failure_telemetry_invoked_once` を追加し、`RootEscalationSink` が telemetry を一度だけ呼ぶことを確認。
- `TracingFailureTelemetry` 用のテストは `#[cfg(feature = "std")]` 内で `tracing_subscriber::fmt::test_writer` を利用し、ログ出力が行われることを検証。
- `thumbv6m-none-eabi` ターゲットで `cargo check` を実行し、`alloc` 依存が存在しないことを確定。
- 並行環境での再入性を検証するため、`telemetry/tests.rs` に複数スレッドからの呼び出しを擬似的に再現するテストを追加。

## ベンチマーク速報値（2025-10-14 実測）
- `cargo bench --bench failure_telemetry --features std`
  - `failure_telemetry_shared`: 776ps〜783ps/呼び出し
  - `failure_telemetry_direct`: 257ps〜258ps/呼び出し
- 共有ラッパ経由のオーバーヘッドは約 3x だが、いずれも sub-ns レベルであり現行のエラーハンドリング頻度では許容範囲と判断。
- 詳細な出力は `docs/perf/2025-10-14-failure-telemetry.md` に記録。

## フェーズ3メモ（進行中）
- `scripts/ci-check.sh no-std` を実行し、`alloc` のみの構成（`thumbv6m-none-eabi` 相当）でも `FailureTelemetryShared` / 観測フックがコンパイル可能であることを確認。（2025-10-14）
- 観測設定が未指定の場合でも `FailureTelemetryObservationConfig::new()` を用いて no-op 動作となるよう初期値を統一。
- `defmt` 連携案を `docs/design/D18-telemetry-defmt-next-actions.md` にまとめ、Builder API を通じた挿入が可能であることを確認。
- `scripts/ci-check.sh all` を完走し、`std` / `no_std` 両構成および関連テストが成功することを確認。（2025-10-14）

## リスクと緩和策
- Telemetry 呼び出しの遅延がスーパーバイザの応答を阻害するリスク → 非同期処理は導入せず、呼び出し時間を計測する `debug_assert!` をフェーズ2で導入。
- `FailureTelemetryShared` 経由のダイナミックディスパッチオーバーヘッド → `#[cfg_attr(test, derive(Debug))]` を検討し、ベンチで実測する（2025-10-20 までに結論）。
- `no_std` 向けに `alloc` を使わない設計を保てないリスク → `feature = "alloc"` の導入は最終手段とし、まず固定長バッファで設計。

## マイルストーン（ドラフト）
- 2025-10-16: RFC 草案を `docs/rfc/2025-10-root-escalation-telemetry.md` として提出。
- 2025-10-18: フェーズ2（Builder 注入）の実装完了、`cargo test --workspace` を通過。
- 2025-10-21: `thumb` ターゲットでの `cargo check` 完了、`no_std` プロファイルの検証をドキュメント化。
- 2025-10-24: テスト戦略に基づく自動テスト追加、`./scripts/ci-check.sh all` 成功ログを残す。
- 2025-10-28: フェーズ3・4完了レビュー、ドキュメント更新を取りまとめて main ブランチへマージ。

## 実装タスク詳細（フェーズ2完了報告）
1. `modules/actor-core/src/api/supervision/telemetry.rs`
   - ✅ `FailureSnapshot` を追加し、`FailureTelemetry` をスナップショット API に移行。
   - ✅ `NoopFailureTelemetry` を `spin::Once` ベースで共有化し、重複確保を解消。
2. `modules/actor-core/src/runtime/system/internal_actor_system.rs`
   - ✅ Config 経由で telemetry を注入し、`InternalActorSystemSettings` へ配線。
3. `modules/actor-core/src/runtime/scheduler/ready_queue_scheduler.rs`
   - ✅ `CompositeEscalationSink` まで telemetry ハンドルを透過させ、Root で消費。
4. テスト追加
   - ✅ `modules/actor-core/src/api/supervision/telemetry/tests.rs` で `FailureSnapshot` のフィールド保証と `TracingFailureTelemetry` のログ検証を実装。
   - ✅ `modules/actor-core/src/api/supervision/escalation.rs` のユニットテストで Telemetry 観測フックが MetricsSink へイベントを送ることを検証。
5. ベンチマーク
   - ✅ `modules/actor-core/benches/failure_telemetry.rs` を追加し、共有ラッパ経由ディスパッチと直接呼び出しのコスト比較を可視化。
6. ドキュメント同期
   - ✅ 本計画書を更新し、フェーズ2の進捗を反映。
7. API 拡張
   - ✅ `TelemetryContext` / `FailureTelemetryBuilderShared` を追加し、`ActorSystemConfig::with_failure_telemetry_builder` および `with_failure_observation_config` を提供。

## 残タスク追跡テンプレート案
- [ ] RFC ドラフト共有（担当: TBD、期限: 2025-10-16）
- [ ] Builder への注入 API 実装（担当: TBD、期限: 2025-10-18）
- [ ] Telemetry テスト拡充（担当: TBD、期限: 2025-10-24）
- [ ] `no_std` 構成での実地検証メモ作成（担当: TBD、期限: 2025-10-21）
- [ ] ドキュメント最終レビューと公開（担当: TBD、期限: 2025-10-28）
