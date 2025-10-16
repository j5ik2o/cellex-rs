# ReceiveTimeout 提供計画 (2025-10-09)

## 現状サマリ (2025-10-13)
- `ReceiveTimeoutScheduler` / `ReceiveTimeoutDriver` 抽象を actor-core に導入し、Tokio 用 `TokioReceiveTimeoutDriver` は GenericActorRuntime から注入できる。
- `Context::set_receive_timeout` / `cancel_receive_timeout` が公開され、Scheduler 経由で `SystemMessage::ReceiveTimeout` が発火する。
- Mailbox 経路ではユーザーメッセージ受信後に `notify_receive_timeout_activity` が呼ばれ、タイマーの自動リセットが行われている。

## 未解決課題
- [MUST] `NotInfluenceReceiveTimeout` マーカーとハンドルを modules 配下に実装し、指定したメッセージでタイマーがリセットされないようにする。
- [MUST] Embedded 向けの `EmbassyReceiveTimeoutDriver` を実装し、`EmbassyActorRuntimeExt::with_embassy_scheduler()` と併せて検証する。
- [SHOULD] Driver / Scheduler の Drop 時にタイマーが確実に停止することを保証するテストを追加する。
- [SHOULD] Builder/API レベルで ReceiveTimeout を設定するためのガイドとサンプルを整備し、利用者ドキュメントを更新する。

## 優先アクション
1. `NotInfluenceReceiveTimeout` トレイト／ハンドルを追加し、Tokio ドライバでの動作テストを整備する。
2. Embassy ベースのドライバ試作を行い、`embedded_rc` / `embedded_arc` 双方でコンパイルと基本動作を確認する。
3. ReceiveTimeout の利用例（set/cancel/再アーム）を含むドキュメントとサンプルコードを追記する。
