# Message Metadata ローカルストア移行（完了）

## サマリ（2025-10-18）
- 旧 `runtime::message::metadata_table`（グローバルサイドテーブル）を廃止し、`UserMessage` が `MetadataStorageRecord` を直接保持する形へ移行した。
- テスト（`modules/actor-core/src/api/actor/tests.rs`）ではメタデータ復元とドロップフックの挙動を検証済み。
- ベンチマーク（`modules/actor-core/benches/metadata_table.rs`）をローカルストア方式で更新し、`inline` 方式との比較が継続可能。
- Remote/Cluster 経路からもローカルストア内容をそのまま直列化する設計へ繋げる前提が整った。

## 主な変更
- `modules/actor-core/src/api/messaging/user_message.rs`
  - `MetadataKey` ベースの API を廃止し、`MetadataStorageRecord` を保持。
  - `into_parts` が `(Message, Option<MessageMetadata<Mode>>)` を直接返す。
- `modules/actor-core/src/api/actor/props.rs` / `ask.rs`
  - 新 API に追従し、グローバルテーブル呼び出しを撤去。
- `modules/actor-core/src/internal/message/metadata_table*.rs`
  - ファイルごと削除。
- `modules/actor-core/src/api/actor/tests.rs`
  - メタデータの破棄確認テストを追加。
- `docs/tasks/D8-message-metadata-local-store.md`
  - TODO リスト化していた作業を完了扱いに移行。

## 今後の対応
- Remote/Cluster 向けシリアライズ仕様にローカルストアの構造を織り込み、優先度や `PriorityChannel` の保持をエンドツーエンドで検証する。
- README / サンプル更新は別チケットで進行すること。

## 参考
- 旧メモ: `docs/design/archive/2025-10-09-message-metadata-refactor.md`
- Priority チャネル整合性計画: `docs/design/D4-priority-channel-next-actions.md`
