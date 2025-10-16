# defmt Telemetry 連携：次アクション

## 優先タスク
1. `FailureTelemetryBuilderShared` を用いた defmt 実装を実際に導入し、platform-tests などで smoke テストを追加する。
2. `no_std` 構成で `alloc` だけを使用することを確認し、依存クレート（panic-probe / defmt-rtt 等）の組み合わせをドキュメント化する。
3. defmt へのメトリクス連携（例: 重要イベントのログ化）を設計し、必要な ObservationConfig の設定例を提示する。

## 参考
- 旧メモは `docs/design/archive/2025-10-14-telemetry-defmt-spike.md` を参照。
