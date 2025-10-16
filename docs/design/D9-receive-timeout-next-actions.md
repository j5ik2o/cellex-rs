# ReceiveTimeout 機能：次アクション

## 優先タスク
1. `NotInfluenceReceiveTimeout` トレイトとハンドルを実装し、指定したメッセージがタイマーをリセットしないようにする。現状のコードベースには未実装（`rg "NotInfluenceReceiveTimeout"` で未ヒット）。
2. `EmbassyReceiveTimeoutDriver` を実装し、`with_embassy_scheduler()` との統合テストを追加する。
3. Driver/Scheduler の Drop 時にタイマーが確実に停止することを保証するテストを整備する。
4. ReceiveTimeout の利用例（set/cancel/再アーム）をドキュメント化し、サンプルを更新する。

## 参考
- 旧メモは `docs/design/archive/2025-10-09-receive-timeout-plan.md` を参照。
