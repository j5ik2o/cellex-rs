# Message Metadata 再設計：次アクション

## 現状メモ
- 旧来のグローバルサイドテーブル `runtime::message::metadata_table` は廃止対象とし、メッセージ単位で完結するローカルメタデータストアへ移行する。
- `PriorityEnvelope<M>` / `MessageEnvelope` 内に `MetadataStore`（仮称）を内包させ、優先度・チャネル・TypeKey など必要情報を一括管理する。
- Remote / Cluster 経路のシリアライズは Envelope 本体に含まれるメタ情報をそのまま送出し、追加の ACK や TTL 管理を不要にする。

## 設計方針（2025-10-18 更新）
- メタデータは Envelope が保持するローカルストアに記録し、Envelope のライフサイクルに従って自動で解放される。
- ストア実装は `no_std` を考慮し、`SmallVec` ベースの固定長スロット（少量想定）と `FxHashMap` ベースのフォールバックを組み合わせる。
- Remote/Cluster 経路では `PriorityChannel` / `priority` / 任意メタを直列化し、受信側で `PriorityEnvelope` を再構築する。
- 旧グローバルテーブル API（`MessageEnvelope::user_with_metadata` など）は非推奨化し、最終的に削除する。

## 優先タスク
1. グローバルテーブル参照箇所を棚卸しし、Envelope 内ストアを利用する形へ段階的に置き換える。
2. Remote / Cluster のシリアライズコードを更新し、ローカルストア内容を欠損なくエンコード／デコードする。
3. `modules/actor-core/benches/metadata_table.rs` を改訂し、ローカルストア方式のベンチマークと比較結果を記録する。
4. 利用者向けドキュメント（README / サンプル）を更新し、ローカルメタデータの API 例とベストプラクティスを明示する。

## 参考
- 旧メモは `docs/design/archive/2025-10-09-message-metadata-refactor.md` を参照。
- Priority チャネル整合性テスト計画：`docs/design/D4-priority-channel-next-actions.md`
