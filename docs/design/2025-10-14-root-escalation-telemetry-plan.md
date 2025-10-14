# RootEscalationSink テレメトリ抽象化計画

最終更新: 2025-10-14（設計メモのみ。実装はこれから）

## 背景
- `modules/actor-core/src/api/supervision/escalation.rs:124` で `tracing::error!` を直接呼び出しており、`#[cfg(feature = "std")]` に依存している。
- リポジトリ方針としてランタイム本体に `std` フラグ分岐を持ち込まないこと、`no_std` 対応を阻害しないことが求められている。
- 既に `event_handler` / `event_listener` という外部通知フックが存在するが、ロギング責務の分離が不十分で、`no_std` でのログ出力手段が確立していない。

## ゴール
1. ランタイム本体から `tracing` 依存と `#[cfg(feature = "std")]` 分岐を除去する。
2. 失敗通知の拡張ポイントを整理し、`std` / `no_std` 双方で運用可能なロギング・モニタリング手段を注入できるようにする。
3. 既存の `event_handler` / `event_listener` の活用方針を明確化し、必要であれば API 調整を行う。

## 現状 / アプローチ概要
- 2025-10-14 時点では `modules/actor-core/src/api/supervision/escalation.rs` に `#[cfg(feature = "std")] use tracing::error;` が残っており、`std` 依存のログが直書きされている。以下はその除去に向けた計画であり、まだ着手していない。

- `FailureTelemetry`（仮称）トレイトを導入し、`FailureInfo` を受け取って副作用を生じさせるインタフェースを定義する。
- `RootEscalationSink` は `FailureTelemetry` を保持し、イベント通知後に呼び出すのみとする。
- `std` 環境では `TracingFailureTelemetry` 実装を提供し、既存の `tracing::error!` ログを移植する。
- `no_std` 環境では `NullFailureTelemetry` または `RingBufferFailureTelemetry`（将来の拡張）などの実装を注入できるようにする。
- ランタイム初期化時に telemetry 実装を注入する API を追加し、既存の `event_handler` と共存させる設計を検討する。

## 実施フェーズ
### フェーズ1: インタフェース整備（未着手）
- `modules/actor-core/src/api/supervision/telemetry.rs`（新規）に `FailureTelemetry` トレイトと標準実装の枠組みを追加。
- `RootEscalationSink` に telemetry フィールドを追加し、処理順序を整理。
- 既存の `event_handler` / `event_listener` 呼び出し順を維持しつつ、`tracing` 直接呼び出しを削除。

### フェーズ2: `std` 向け実装と注入経路（未着手）
- `cfg(feature = "std")` ではなく、クレートレベルの `pub fn set_default_failure_telemetry(...)` 等で実装を注入。
- `std` 環境用に `TracingFailureTelemetry` を提供 (`modules/actor-core/src/api/supervision/telemetry/tracing.rs` 想定)。
- 既存の `RootEscalationSink::new` がデフォルト telemetry を利用するよう調整。

### フェーズ3: `no_std` 運用確認（未着手）
- `NullFailureTelemetry` を用いた `no_std` ビルド（thumb ターゲット）確認。
- 追加で、`defmt` などのバックエンドを後から導入しやすい構造の検討メモを追記。

### フェーズ4: ドキュメントとテスト（未着手）
- `docs/design` に最終設計メモを追記、本ファイルを更新。
- ユニットテスト／統合テストで `FailureTelemetry` の呼び出し順・副作用を検証。
- `cargo fmt`, `cargo clippy --workspace --all-targets`, `cargo test --workspace` を実行し、CI チェックに備える。

## オープン課題
- Telemetry の注入ポイントをどこに置くか（`ActorSystemBuilder` 相当の API があるか要確認）。
- `event_handler` / `event_listener` と telemetry の責務分割（重複を避けるための再設計が必要か）。
- `no_std` 環境でのログ出力先（UART, RTT, defmt 等）を採用するか、利用者に委ねるか。
- `FailureInfo` の clone コスト最適化（大量発生時のパフォーマンス影響評価）。

## 次アクション
1. Telemetry 抽象の API 骨子案を小さな RFC としてまとめ、レビューを募る。
2. フェーズ1と2を同ブランチで実装してレビューに出す（段階的にビルドが壊れないように注意）。
3. `./scripts/ci.sh all` での一括確認（thumb ターゲット含む）を必須とする。
