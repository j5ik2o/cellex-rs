# FailureTelemetry ベンチマーク結果（2025-10-14）

- 計測日時: 2025-10-14
- 計測環境: `cargo bench --bench failure_telemetry --features std`
- コマンド:

```bash
cargo bench --bench failure_telemetry --features std
```

## 計測値

| ケース | 平均時間 (ps) | 95% 信頼区間 | 備考 |
| --- | --- | --- | --- |
| failure_telemetry_shared | 779 ± 4 | [775.68, 783.23] | `FailureTelemetryShared::with_ref` 経由で `NoopFailureTelemetry` を呼び出し |
| failure_telemetry_direct | 258 ± 2 | [256.59, 258.29] | トレイト実装を直接呼び出し |

> `criterion` 出力より抜粋。Plotters backend を使用（gnuplot 非インストール）。

## 所見

- 共有ラッパ経由のオーバーヘッドは約 3x だが、絶対値はサブナノ秒級。
- 共有ラッパの利便性（DI や追跡用フック）を維持したまま十分許容できるレイテンシと判断。
- 今後、メトリクス観測フックを追加する際は本結果と比較し、オーバーヘッドの増加率を評価する。
- `scripts/ci-check.sh no-std` による `alloc` 構成チェックも通過済み（2025-10-14）。
