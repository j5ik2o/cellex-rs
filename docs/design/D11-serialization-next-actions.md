# シリアライゼーション戦略：次アクション

## 現状メモ
- `SerializerRegistryExtension` が built-in serializer/binding を登録し、`SerializationRouter` を ActorSystem 起動時に利用可能な状態にしている。
- `TypeKey` マクロとテストが存在し、型キー定義はライブラリ側で整備済み。

## 優先タスク
1. ActorSystem 外部からのバインディング更新／再読み込み API を整理し、ドキュメント化する。
2. Remote / Cluster 経路で `SerializationRouter` を利用し、エンドツーエンドのシリアライズ／デシリアライズを検証する統合テストを追加する。
3. 型キー未登録時のフォールバックポリシー（例: JSON デフォルト）を実装または仕様化する。
4. 公式ガイド／サンプルに TypeKey 定義とバインディングフローを追記する。

## 参考
- 旧メモは `docs/design/archive/2025-10-11-serialization-strategy.md` を参照。
