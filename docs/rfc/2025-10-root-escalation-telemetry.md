# RootEscalationSink Telemetry 拡張 RFC（ドラフト）

- 作成日: 2025-10-14
- 著者: TBD
- ステータス: Draft
- 対象リリース: 未定（pre-release フェーズ）

## 背景

RootEscalationSink に導入した `FailureTelemetry` 抽象は、スナップショット API と共有ラッパ (`FailureTelemetryShared`) により std / no_std 間の統一を確保した。しかし今後の運用では以下の拡張が求められている。

- `FailureSnapshot` にメタタグや監査用 ID を付与し、外部システムと相互運用を進める。
- Telemetry の呼び出し頻度やレイテンシを可視化するためのメトリクスフックを導入する。
- アプリケーション開発者が Builder API から telemetry を差し替えやすくするための DX 改善を行う。

本 RFC では上記要求を整理し、段階的な実装計画を提示する。

## 目的

1. `FailureSnapshot` に可変長タグ群を追加し、アプリケーション側で任意のキー/値情報を付与できるようにする。
2. Telemetry 呼び出し経路に累積処理時間や呼び出し回数を観測する仕組みを導入する。
3. `ActorSystemConfig` / `RootEscalationSink` へ Builder API を追加し、初期化時の拡張ポイントを明示化する。

## 要件

- **タグ拡張**: `FailureSnapshot` に `TelemetryTag` の固定長バッファ（当初は 4 個）を追加し、ヒープ確保を伴わない形で key/value を格納できるようにする。no_std 環境では `heapless::Vec` 相当を検討し、std では `SmallVec` 互換型を導入する。
- **メトリクス**: Telemetry 呼び出し時間を `MetricsSink` に記録するオプションフックを追加し、feature flag ではなく runtime 設定で ON/OFF 可能にする。
- **DX 改善**: `FailureTelemetryBuilder` (仮称) を設計し、`ActorSystemConfig::with_failure_telemetry_builder` 経由で初期化するパターンを提供する。
- **互換性**: 現在の `FailureTelemetry` トレイトは破壊的変更が許容されるが、呼び出しシグネチャは極力維持する。必要なら `FailureTelemetryExt` などの追加トレイトで拡張。

## 提案概要

1. `TelemetryTag` 型の導入
   - `TelemetryKey` / `TelemetryValue` を `&'static str` もしくは `Cow<'static, str>` で表現。
   - `FailureSnapshot` に `[Option<TelemetryTag>; 4]` を組み込み、将来的に長さを変更できるよう定数化。
   - `FailureInfo` 生成時に `FailureMetadata` からタグを抽出するアダプタを用意。
   - **API スケッチ**:
     ```rust
     pub struct TelemetryTag {
       pub key: TelemetryKey,
       pub value: TelemetryValue,
     }

     pub type TelemetryKey = Cow<'static, str>;
     pub type TelemetryValue = Cow<'static, str>;

     impl FailureSnapshot {
       pub fn tags(&self) -> &[TelemetryTag];
     }
     ```

2. メトリクス観測フック
   - `FailureTelemetryShared::with_ref` の呼び出し前後で `Instant::now()` を取得し、差分を `MetricsEvent::TelemetryInvoked`（新設）として記録。
   - Overhead を避けるため、観測フックは `FailureTelemetryObservationConfig` で切り替え可能にする。
   - **API スケッチ**:
     ```rust
     pub struct FailureTelemetryObservationConfig {
       pub metrics: Option<MetricsSinkShared>,
       pub capture_timing: bool,
     }

     impl FailureTelemetryShared {
       pub fn with_ref_observed<R>(&self, cfg: &FailureTelemetryObservationConfig, f: impl FnOnce(&dyn FailureTelemetry) -> R) -> R;
     }
     ```

3. Builder API
   - `ActorSystemConfig::with_failure_telemetry_builder` を追加し、`
     Fn(TelemetryContext) -> FailureTelemetryShared` を受け取る。
   - `RootEscalationSink` には `apply_builder` メソッドを追加し、Builder に環境情報（Runtime 拡張、MetricsSink 等）を渡せるようにする。
   - `TelemetryContext` は `alloc` のみで動作し、`no_std` ターゲットでも同一 API を利用できるようにする。
   - **API スケッチ**:
     ```rust
     pub struct TelemetryContext<'a> {
       pub metrics: Option<&'a MetricsSinkShared>,
       pub extensions: &'a Extensions,
     }

     pub type FailureTelemetryBuilder = dyn Fn(&TelemetryContext) -> FailureTelemetryShared + Send + Sync + 'static;

     impl<R> ActorSystemConfig<R> {
       pub fn with_failure_telemetry_builder(mut self, builder: ArcShared<FailureTelemetryBuilder>) -> Self;
     }

     impl<M, R> RootEscalationSink<M, R> {
       pub fn apply_builder(&mut self, ctx: &TelemetryContext, builder: &FailureTelemetryBuilder);
     }
     ```

## 実装フェーズ（案）

1. **フェーズ A**: `FailureSnapshot` のタグ拡張 + 単純な `TelemetryTag` API 実装
2. **フェーズ B**: メトリクス観測フックと `FailureTelemetryObservationConfig`
3. **フェーズ C**: Builder API 導入と `ActorSystemConfig` / `RootEscalationSink` の改修
4. **フェーズ D**: ドキュメント & ベンチマーク更新、アプリケーション側へのガイド提供

## 未解決事項

- `TelemetryTag` の最大件数を動的に確保するか固定長で維持するか。
- 観測フックのオーバーヘッドが許容範囲内か（ベンチマーク実装が必要）。
- Builder API の命名規約（`with_failure_telemetry_builder` vs `configure_failure_telemetry`）。

## 参考情報

- ベンチ結果: `failure_telemetry_shared ≈ 780ps`, `failure_telemetry_direct ≈ 257ps`
- 既存ドキュメント: `docs/design/D17-root-escalation-telemetry-next-actions.md`

---

※本ドラフトはレビュー前提であり、意見・追記歓迎です。
