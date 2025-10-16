# protoactor-go 基本機能パリティ：次アクション

## 優先タスク
1. `NotInfluenceReceiveTimeout` マーカー／ハンドルを実装し、Tokio および Embeded ドライバでタイマー制御を検証する（現状コードベースに未実装）。
2. Behavior DSL にライフサイクル Signal（`Started` / `Stopping` / `Stopped` / `Restarting`）と stackable `become` を提供し、protoactor-go と同等の挙動を確認する。
3. `Context::watch` / `unwatch` / `stop` / `poison` など監視・停止 API を公開し、SystemMessage の配信保証テストを整備する。
4. GuardianStrategy のバリエーション（OneForOne / AllForOne / ExponentialBackoff 等）を提供し、テストとドキュメントを整備する。
5. `ProcessRegistry` 相当の PID 解決と DeadLetter 連携を設計し、remote モジュールとの整合性を検証する。

## 参考
- 旧メモは `docs/design/archive/2025-10-09-basic-feature-parity.md` を参照。
