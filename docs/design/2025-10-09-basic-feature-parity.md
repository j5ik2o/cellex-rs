# 基本機能パリティ TODO（protoactor-go 対比）

## 現状サマリ (2025-10-13)
- Ask 系 API（`request*`, `AskFuture`, `respond` など）は typed センダー経由で統合済み。
- ReceiveTimeout 抽象と `Context::set_receive_timeout` / `cancel_receive_timeout` は実装され、Tokio ランタイム向けドライバも導入済み。
- Guardian と EscalationSink は core 側に集約され、監視者リストの自動管理が動作している。

## 未解決課題
- [MUST] ReceiveTimeout: `NotInfluenceReceiveTimeout` マーカー／ハンドルを `modules/` 配下に実装し、ユーザーメッセージからタイマーを制御できるようにする。Embedded 用ドライバ（Embassy）と統合テストも未整備。
- [MUST] ライフサイクルイベント（`Started`, `Stopping`, `Stopped`, `Restarting` 等）と `become_stacked` 互換 API を Behavior DSL に追加する。
- [MUST] `Context::watch` / `unwatch` / `stop` / `poison` など監視・停止 API を公開し、SystemMessage の配信保証テストを整備する。
- [SHOULD] `OneForOneStrategy` / `AllForOneStrategy` / `ExponentialBackoffStrategy` 等のスーパーバイザ戦略を GuardianStrategy 実装として提供する。
- [SHOULD] `ProcessRegistry` 相当の仕組み（PID 解決と DeadLetter 連携）を Rust 版に設計し、remote との互換性を確認する。

## 優先アクション
1. ReceiveTimeout まわりのマーカー型とハンドル API を追加し、Tokio ドライバのリグレッションテストを更新する。
2. Behavior DSL へライフサイクル Signal と stackable `become` を実装し、protoactor-go とのパリティテストを追加する。
3. 監視／停止 API とスーパーバイザ戦略の実装計画を具体化し、順次 PR に分割して着手する。
