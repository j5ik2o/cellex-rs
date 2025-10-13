//! [`prost`] を利用した Protobuf シリアライザ実装。

#![deny(missing_docs)]

use cellex_serialization_core_rs::error::{DeserializationError, SerializationError};
use cellex_serialization_core_rs::id::SerializerId;
use cellex_serialization_core_rs::impl_type_key;
use cellex_serialization_core_rs::message::SerializedMessage;
use cellex_serialization_core_rs::serializer::Serializer;
use cellex_utils_core_rs::sync::ArcShared;
use prost::Message;

/// `prost` 用に予約されたシリアライザ ID。
pub const PROST_SERIALIZER_ID: SerializerId = SerializerId::new(2);

/// `prost` をバックエンドとするシリアライザ。
#[derive(Debug, Clone, Default)]
pub struct ProstSerializer;

impl ProstSerializer {
  /// 新しいインスタンスを生成します。
  #[must_use]
  pub const fn new() -> Self {
    Self
  }

  /// 値をシリアライズして [`SerializedMessage`] を返します。
  pub fn serialize_message<T>(
    &self,
    type_name: Option<&str>,
    value: &T,
  ) -> Result<SerializedMessage, SerializationError>
  where
    T: Message, {
    let mut buffer = Vec::new();
    value
      .encode(&mut buffer)
      .map_err(|err| SerializationError::custom(err.to_string()))?;
    self.serialize_with_type_name_opt(buffer.as_slice(), type_name)
  }

  /// メッセージを指定した型にデシリアライズします。
  pub fn deserialize_message<T>(&self, message: &SerializedMessage) -> Result<T, DeserializationError>
  where
    T: Message + Default, {
    T::decode(&*message.payload).map_err(|err| DeserializationError::custom(err.to_string()))
  }
}

impl Serializer for ProstSerializer {
  fn serializer_id(&self) -> SerializerId {
    PROST_SERIALIZER_ID
  }

  fn content_type(&self) -> &str {
    "application/protobuf"
  }

  fn serialize_with_type_name_opt(
    &self,
    payload: &[u8],
    type_name: Option<&str>,
  ) -> Result<SerializedMessage, SerializationError> {
    // Protobuf は型情報が無いと検証できないため、そのままバイナリを保持する。
    let mut message = SerializedMessage::new(self.serializer_id(), payload.to_vec());
    if let Some(name) = type_name {
      message.set_type_name(name);
    }
    Ok(message)
  }

  fn deserialize(&self, message: &SerializedMessage) -> Result<Vec<u8>, DeserializationError> {
    Ok(message.payload.clone())
  }
}

/// 共有シリアライザインスタンスを生成します。
#[must_use]
pub fn shared_prost_serializer() -> ArcShared<ProstSerializer> {
  ArcShared::new(ProstSerializer::new())
}

/// Marker type representing Prost-encoded payload bindings.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProstTypeKey;

impl_type_key!(ProstTypeKey, "cellex.serializer.prost", PROST_SERIALIZER_ID);
