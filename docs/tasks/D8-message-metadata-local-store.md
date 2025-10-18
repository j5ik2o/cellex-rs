# D8: メッセージローカルメタデータ移行タスク一覧

## 背景
- グローバルサイドテーブル `runtime::message::metadata_table` を廃止し、Envelope 内で完結するローカルストアへ移行する。
- Remote / Cluster 経路で優先度および `PriorityChannel` を確実に再構築し、Control レーンの整合性を担保する。

## TODO
1. 参照箇所棚卸し
   - `MessageEnvelope::user_with_metadata` など既存 API の呼び出し元を列挙し、置換順序を決める。
2. Envelope 拡張実装
   - `PriorityEnvelope<M>` / 基本 Envelope にローカルストアを追加し、優先度・チャネル・任意メタを格納できるようにする。
3. シリアライズ更新
   - Remote/Cluster 向けシリアライズでローカルストア内容を送受信できるよう protobuf/gRPC 等の定義を調整する。
4. 旧 API の段階的削除
   - グローバルテーブル API を非推奨化し、最終的に実装ごと除去する。
5. ベンチマーク刷新
   - `modules/actor-core/benches/metadata_table.rs` をローカルストア方式に合わせて更新し、性能比較を取得する。
6. ドキュメント更新
   - README・サンプル・設計ドキュメントにローカルストアの利用手順とベストプラクティスを追記する。

## リンク
- 設計メモ: `docs/design/D8-message-metadata-next-actions.md`
- 優先タスク: `docs/design/D4-priority-channel-next-actions.md`
