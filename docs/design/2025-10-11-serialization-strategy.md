# Serialization Routing Strategy (2025-10-11)

## 背景
- `cellex-serialization-core-rs` は `SerializerId → Serializer` マッピングのみを提供し、メッセージ型とシリアライザの関連付けは利用側任せになっている。
- Akka/Pekko では `Serialization` サービスが型に応じてシリアライザを自動解決するため、呼び出し側の負担が少ない。
- cellex でも JSON / Protobuf など複数フォーマットを扱い始めており、今後も `postcard` などの追加が想定されるため、自動ルーティング機構を整備したい。

## 目標
1. **型 → SerializerId** のマッピングを公式サポートする。
2. 呼び出し側はメッセージ型とバイト列を渡すだけで、自動的に登録済みシリアライザが選択されるようにする。
3. `no_std` / `std` 双方で利用可能な設計とし、`JSON` や `Prost` といった `std` 依存コンポーネントはフィーチャ分岐に委ねる。

## 想定コンポーネント

| コンポーネント | 役割 |
| --- | --- |
| `SerializerRegistry` (既存) | `SerializerId → Serializer` の登録と解決を担当。 |
| `TypeBindingRegistry` (新規) | 型キー → `SerializerId` のマッピングを管理。 |
| `SerializationRouter` (新規) | 利用側 API。型キーを受け取り、`TypeBindingRegistry` と `SerializerRegistry` を組み合わせてシリアライズ／デシリアライズを実行。 |

## 提案する API スケッチ

```rust
pub trait TypeKey {
  fn type_key() -> &'static str;
}

pub struct TypeBindingRegistry {
  // BTreeMap<String, SerializerId>
}

impl TypeBindingRegistry {
  pub fn bind(&self, key: &str, serializer: SerializerId) -> Result<(), RegistryError>;
  pub fn resolve(&self, key: &str) -> Option<SerializerId>;
}

pub struct SerializationRouter {
  bindings: TypeBindingRegistry,
  serializers: InMemorySerializerRegistry,
}

impl SerializationRouter {
  pub fn serialize<T: TypeKey + SerializeLike>(
    &self,
    value: &T,
  ) -> Result<SerializedMessage, SerializationError>;

  pub fn deserialize<T: TypeKey + DeserializeLike>(
    &self,
    message: &SerializedMessage,
  ) -> Result<T, DeserializationError>;
}
```

`SerializeLike` / `DeserializeLike` は `serde` / `prost` / `postcard` など異なる実装を抽象化するため、バッキングシリアライザ側でトレイトを差し替える想定。初期バージョンでは `Serialize` / `DeserializeOwned` に限定する案もあり。

## 実装ステップ
1. `TypeBindingRegistry` を `serialization-core` に追加し、シンプルな `BTreeMap<String, SerializerId>` を管理する。`no_std` + `alloc` に対応。
2. `SerializationRouter` を `serialization-core` に実装し、既存の `InMemorySerializerRegistry` インスタンスを内部で使用する。
3. `SerdeJsonSerializer` / `ProstSerializer` など各フォーマットクレートで `TypeKey` 実装のためのヘルパーを提供（例: `pub fn type_key_for<T>() -> &'static str`）。
4. `actor-core` の `SerializerRegistryExtension` で、標準シリアライザ登録時に `TypeBindingRegistry` へデフォルトバインディングを追加（たとえば JSON をデフォルトにする、あるいは利用者が設定できるようフックを公開）。
5. 将来的には `Configuration`（toml/yaml 等）からバインディングを読み込めるようにする余地を残す（Akka の設定ファイル相当）。

## 追跡事項
- `TypeKey` の実装を自動派生するマクロや derive の提供。
- `SerializedMessage` に格納する `type_name` との整合性（`TypeKey::type_key()` を格納すれば、受信側で照合できる）。
- 既存コードに影響を与えない導入パス（新 API を opt-in にする）。
