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
- `modules/actor-core/src/api/supervision/telemetry.rs` を新設し、`FailureTelemetry` トレイトおよび `NullFailureTelemetry` / `TracingFailureTelemetry` を定義。
- `RootEscalationSink` は telemetry フィールドを保持し、`default_failure_telemetry()` によりビルド構成に応じた実装（`std,unwind-supervision` なら tracing、それ以外は null）を利用する。`tracing::error!` の直書きは解消済み。
- telemetry を外部から注入するための setter を公開済み。ただし `ActorSystem`/`PriorityScheduler` など高レベル API ではまだ利用手段を整備していない。
- 今後は初期化時のデフォルト注入や、アプリケーション側で telemetry を差し替えるビルダー API の追加を検討する。

- `FailureTelemetry`（仮称）トレイトを導入し、`FailureInfo` を受け取って副作用を生じさせるインタフェースを定義する。
- `RootEscalationSink` は `FailureTelemetry` を保持し、イベント通知後に呼び出すのみとする。
- `std` 環境では `TracingFailureTelemetry` 実装を提供し、既存の `tracing::error!` ログを移植する。
- `no_std` 環境では `NullFailureTelemetry` または `RingBufferFailureTelemetry`（将来の拡張）などの実装を注入できるようにする。
- ランタイム初期化時に telemetry 実装を注入する API を追加し、既存の `event_handler` と共存させる設計を検討する。

## 実施フェーズ
### フェーズ1: インタフェース整備（完了）
- `FailureTelemetry` トレイトおよび `NullFailureTelemetry` / `TracingFailureTelemetry` を追加。
- `RootEscalationSink` が telemetry を保持し、`tracing` 直接呼び出しを削除済み。

### フェーズ2: `std` 向け実装と注入経路（進行中）
- `TracingFailureTelemetry` を提供済みだが、`ActorSystem` など上位 API で自動注入する仕組みは未整備。
- 目標は `ActorSystemBuilder`（または相当する初期化 API）から telemetry を設定できるようにし、`std` 構成では tracing ベースの実装をデフォルトにすること。

### フェーズ3: `no_std` 運用確認（未着手）
- `NullFailureTelemetry` での運用確認は必要（`thumb` チェックは `./scripts/ci.sh all` に任せる）。
- `defmt` 等への拡張案を別途検討する。

### フェーズ4: ドキュメントとテスト（未着手）
- `docs/design` に最終設計メモを追記、本ファイルを更新。
- ユニットテスト／統合テストで `FailureTelemetry` の呼び出し順・副作用を検証。
- `./scripts/ci.sh all` を実行し、CI チェックに備える。

## オープン課題
- Telemetry の注入ポイントをどこに置くか（`ActorSystemBuilder` 相当の API があるか要確認）。
- `event_handler` / `event_listener` と telemetry の責務分割（重複を避けるための再設計が必要か）。
- `no_std` 環境でのログ出力先（UART, RTT, defmt 等）を採用するか、利用者に委ねるか。
- `FailureInfo` の clone コスト最適化（大量発生時のパフォーマンス影響評価）。

## 次アクション
1. Telemetry 抽象の API 骨子案を小さな RFC としてまとめ、レビューを募る。
2. フェーズ1と2を同ブランチで実装してレビューに出す（段階的にビルドが壊れないように注意）。
3. `./scripts/ci.sh all` での一括確認（thumb ターゲット含む）を必須とする。
