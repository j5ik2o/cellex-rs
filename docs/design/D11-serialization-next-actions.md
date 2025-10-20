# シリアライゼーション戦略：次アクション

## 現状メモ
- `SerializerRegistryExtension` が built-in serializer/binding を登録し、`SerializationRouter` を ActorSystem 起動時に利用可能な状態にしている。
- `TypeKey` マクロとテストが存在し、型キー定義はライブラリ側で整備済み。

## 優先タスク
1. ActorSystem 外部からのバインディング更新／再読み込み API を整理し、ドキュメント化する。
2. 公式ガイド／サンプルに TypeKey 定義とバインディングフローを追記する。

## 完了済みタスク（2025-10-20）
- `SerializationRouter` にフォールバック解決機構を追加し、`SerializerRegistryExtension` で JSON フォールバックを既定設定化。
- Remote 経路: `modules/remote-core/src/tests.rs` にフォールバック経由で `RemoteRouterPayload` を復元する統合テストを追加。
- Cluster 経路: `modules/cluster-core/src/tests.rs` にフォールバックを利用した `FailureEvent` 伝搬テストを追加し、ローカル／リモート双方での復元を確認。

## 参考
- 旧メモは `docs/design/archive/2025-10-11-serialization-strategy.md` を参照。
