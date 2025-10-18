//! serde_json ベースのシリアライザ実装。

#![deny(missing_docs)]

use cellex_serialization_core_rs::{
  error::{DeserializationError, SerializationError},
  id::SerializerId,
  impl_type_key,
  message::SerializedMessage,
  serializer::Serializer,
};
use cellex_utils_core_rs::sync::ArcShared;
use serde::{de::DeserializeOwned, Serialize};

/// `serde_json` 用に予約されたシリアライザ ID。
pub const SERDE_JSON_SERIALIZER_ID: SerializerId = SerializerId::new(1);

/// `serde_json` バックエンドのシリアライザ実装。
#[derive(Debug, Clone, Default)]
pub struct SerdeJsonSerializer;

impl SerdeJsonSerializer {
  /// 新しいインスタンスを生成します。
  #[must_use]
  pub const fn new() -> Self {
    Self
  }

  /// 値をシリアライズし [`SerializedMessage`] を生成します。
  pub fn serialize_value<T>(
    &self,
    type_name: Option<&str>,
    value: &T,
  ) -> Result<SerializedMessage, SerializationError>
  where
    T: Serialize, {
    let payload = serde_json::to_vec(value).map_err(|err| SerializationError::custom(err.to_string()))?;
    self.serialize_with_type_name_opt(payload.as_slice(), type_name)
  }

  /// メッセージを指定した型にデシリアライズします。
  pub fn deserialize_value<T>(&self, message: &SerializedMessage) -> Result<T, DeserializationError>
  where
    T: DeserializeOwned, {
    serde_json::from_slice(&message.payload).map_err(|err| DeserializationError::custom(err.to_string()))
  }
}

impl Serializer for SerdeJsonSerializer {
  fn serializer_id(&self) -> SerializerId {
    SERDE_JSON_SERIALIZER_ID
  }

  fn content_type(&self) -> &str {
    "application/json"
  }

  fn serialize_with_type_name_opt(
    &self,
    payload: &[u8],
    type_name: Option<&str>,
  ) -> Result<SerializedMessage, SerializationError> {
    serde_json::from_slice::<serde_json::Value>(payload).map_err(|err| SerializationError::custom(err.to_string()))?;
    let mut message = SerializedMessage::new(self.serializer_id(), payload.to_vec());
    if let Some(name) = type_name {
      message.set_type_name(name);
    }
    Ok(message)
  }

  fn deserialize(&self, message: &SerializedMessage) -> Result<Vec<u8>, DeserializationError> {
    serde_json::from_slice::<serde_json::Value>(&message.payload)
      .map_err(|err| DeserializationError::custom(err.to_string()))?;
    Ok(message.payload.clone())
  }
}

/// 共有シリアライザインスタンスを生成します。
#[must_use]
pub fn shared_json_serializer() -> ArcShared<SerdeJsonSerializer> {
  ArcShared::new(SerdeJsonSerializer::new())
}

/// Marker type representing JSON payloads within the serialization router.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonTypeKey;

impl_type_key!(JsonTypeKey, "cellex.serializer.json", SERDE_JSON_SERIALIZER_ID);
