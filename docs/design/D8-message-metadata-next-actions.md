# Message Metadata 再設計：次アクション

## 現状メモ
- グローバルサイドテーブル `runtime::message::metadata_table` は導入済みで、`MessageEnvelope::user_with_metadata` などから利用されている。
- ベンチマーク `modules/actor-core/benches/metadata_table.rs` で inline 保存との比較が実施できる状態。

## 優先タスク
1. `ActorContext` / `Scheduler` などで `InternalMessageMetadata` が露出している箇所を棚卸しし、必要であれば追加の抽象レイヤを導入する。
2. サイドテーブル導入後の Ask/Respond ベンチマークを更新し、結果をドキュメントに反映する。
3. 利用者向けドキュメント（README / サンプル）に typed メタデータの扱い方を追記する。

## 参考
- 旧メモは `docs/design/archive/2025-10-09-message-metadata-refactor.md` を参照。
