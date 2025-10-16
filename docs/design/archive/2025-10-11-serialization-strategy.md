# Serialization Routing Strategy (2025-10-11)

## 現状サマリ (2025-10-13)
- `TypeBindingRegistry` と `SerializationRouter` を `serialization-core` に追加し、型キーから `SerializerId` を解決できる基盤が整った。
- `SerializationRouter::resolve_serializer` により、バインディング済みシリアライザの検証用テストが通過している。
- しかし actor-core / runtime 側ではまだルーターを利用しておらず、デフォルトバインディングの登録も未着手。

## 未解決課題
- [MUST] `SerializerRegistryExtension` から `TypeBindingRegistry` へデフォルトバインディングを追加し、ActorSystem 起動時に自動登録できるようにする。
- [MUST] `TypeKey` 実装（または derive マクロ）を用意し、利用者が型キーを簡単に定義できるようにする。
- [SHOULD] 型キー未登録時のエラーハンドリングとフォールバック方針（例: JSON へフォールバック）を決定し、API とドキュメントに落とし込む。
- [SHOULD] コンフィグファイルやコードからバインディングを更新するための API を整備し、リロード手順を設計する。
- [MUST] ルーターを利用したエンドツーエンドのシリアライズ／デシリアライズテストを追加する。

## 優先アクション
1. ActorSystem 初期化処理で `SerializationRouter` を組み込み、既存のシリアライザ登録フローと連携させる。
2. `TypeKey` の derive／ヘルパ関数を実装し、Serde JSON / Prost 両方のサンプルを用意する。
3. ルーター経由のシリアライズ／デシリアライズ統合テストを作成し、CI に追加する。
