# Priority Channel 管理：次アクション

## 優先タスク
1. `PriorityEnvelope::from_system` の優先度テーブルを網羅的に検証するテストを追加する。現状は `modules/actor-core/src/tests.rs` の最小テストのみで、各 `SystemMessage` の優先度差分を確認できていない。
2. ユーザーメッセージ側から Control チャネルを選択する公式 API を検討する。
3. Remote / Cluster 経路で Control チャネル情報が保持されることを確認する統合テストを用意する。

## 設計方針（2025-10-18 更新）
- **インターフェース整合**: Remote/Cluster 向けの転送フォーマット（仮称 `RemoteEnvelope`）に `priority: i8` と `channel: PriorityChannel` を必須フィールドとして定義し、`PriorityEnvelope` へ往復できることを保証する。
- **RemoteCodec 実装**: `remote-core` に `codec` モジュールを追加し、`RemoteEnvelope<MessageEnvelope<SerializedMessage>>` と `SerializedMessage`（シリアライザ出力）間を相互変換する `RemoteMessageFrame` を導入。これによりヘッダではなく構造体レベルで Control チャネルと優先度を保持できる。
- **protoactor-go との比較**: Go 実装では `remote/endpoint_manager` が `SystemMessage` を送る際に `Header("Proto.Control", true)` を付与して Mailbox に届ける。これと同様に、Rust 版でも Control チャネル情報はシリアライズ対象に含め、Mailbox まで透過させる。
- **Akka/Pekko との比較**: Akka/Pekko の `Envelope` は `system` フラグと優先度をワイヤフォーマットに載せ、受信後に `PriorityMailbox` へ復元する。Rust 版でも `PriorityChannel::Control` の維持をワイヤ仕様に取り込む。
- **ローカルストア連携**: D8 で導入したローカルメタデータ方式を尊重し、優先度・チャネルは Envelope 固有フィールドとして保持しつつ、既存メタ情報は `MessageEnvelope` のローカルストアに格納する。
- **統合テスト計画**:
  1. Remote 経路の擬似実装を用意し、`PriorityEnvelope::from_system` から生成した制御メッセージをシリアライズ→デシリアライズして `PriorityChannel::Control` と優先度が一致することを確認する（`protoactor-go` の `remoting/remote_deliveries_test` 相当を参考）。
  2. Cluster 側では `ClusterFailureBridge` と `RemoteFailureNotifier` を組み合わせ、`fan_out` 時に Control チャネルが維持されるかをテストする（Akka の `SystemMessageDeliverySpec` の検証粒度に倣う）。
- **API 提供**: `MessageEnvelope` に `into_priority_envelope` / `into_control_envelope` / `control_user` を追加し、ユーザーメッセージから公式に Control チャネルを利用できる入口を提供する。
- **CI 反映**: 上記テストは `remote-core` / `cluster-core` の `std` feature 下で実行できるようにし、`scripts/ci.sh all` に含める。

## 参考
- 旧メモは `docs/design/archive/2025-10-07-priority-channel-table.md` を参照。
