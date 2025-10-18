# D8: メッセージローカルメタデータ移行タスク（完了）

## サマリ
- グローバルサイドテーブルを撤去し、`UserMessage` がローカルメタデータを保持する実装へ移行した。
- `Props` / `ask` / テスト / ベンチマークを更新し、旧 API 依存を排除済み。
- 設計ドキュメントおよびワークノートに最終状態を記録した。

## 残タスク（別チケットへ移管）
- Remote/Cluster シリアライザでローカルメタデータを取り扱う仕様の確定。
- README / サンプルのドキュメント更新。

## リンク
- アーカイブ: `docs/design/archive/2025-10-18-message-metadata-local-store.md`
- 関連: `docs/design/D4-priority-channel-next-actions.md`
