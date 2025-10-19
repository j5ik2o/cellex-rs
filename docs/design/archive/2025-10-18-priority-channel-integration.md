# Priority Channel 管理（完了記録）

## サマリ（2025-10-18）
- Remote/Cluster 経路に `RemoteEnvelope` / `RemoteMessageFrame` を導入し、`priority` と `PriorityChannel` をワイヤフォーマットで保持できるようにした。
- `MessageEnvelope` に Control チャネル公式 API（`into_control_envelope` / `control_user` など）を追加し、ユーザーメッセージからの制御経路選択を容易化。
- `SystemMessage` の優先度テーブルを網羅テストし、値の回帰を防止。
- `scripts/ci-check.sh` に `cellex-remote-core-rs` のテストを追加し、Remote 経路の検証を CI に組み込んだ。

## 主な変更
- `modules/remote-core/src/remote_envelope.rs` / `codec.rs`
  - `RemoteEnvelope` の `into_parts_with_channel` を追加。
  - `RemoteMessageFrame` と `frame_from_serialized_envelope` / `envelope_from_frame` を実装。
- `modules/remote-core/src/tests.rs`
  - システム／ユーザーメッセージのラウンドトリップテストを追加。
- `modules/actor-core/src/api/messaging/message_envelope.rs`
  - Control チャネル API とユニットテストを追加。
- `modules/actor-core/src/api/mailbox/messages/system_message.rs`
  - 優先度テーブル網羅テストを追加。
- `scripts/ci-check.sh`
  - remote-core テストを `run_std` に追加。

## 今後の検討事項
- Remote/Cluster 実装での `SerializedMessage` デシリアライズ（実際の SerializerRouter との統合）は別タスクで継続。
- Cluster 経路の実装が進んだ段階で、実インテグレーションテストを追加する。

## 参考
- 旧メモ: `docs/design/archive/2025-10-07-priority-channel-table.md`
- RemoteCodec 実装: `modules/remote-core/src/codec.rs`
