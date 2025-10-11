//! `postcard` ベースのシリアライザ実装。

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;

use alloc::string::ToString;
use cellex_serialization_core_rs::error::{DeserializationError, SerializationError};
use cellex_serialization_core_rs::id::SerializerId;
use cellex_serialization_core_rs::message::SerializedMessage;
use cellex_serialization_core_rs::serializer::Serializer;
use cellex_utils_core_rs::sync::ArcShared;
use serde::{de::DeserializeOwned, Serialize};

/// `postcard` 用に予約されたシリアライザ ID。
pub const POSTCARD_SERIALIZER_ID: SerializerId = SerializerId::new(3);

/// `postcard` をバックエンドとするシリアライザ。
#[derive(Debug, Clone, Default)]
pub struct PostcardSerializer;

impl PostcardSerializer {
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
    T: Serialize, {
    let payload = postcard::to_allocvec(value).map_err(|err| SerializationError::custom(err.to_string()))?;
    self.serialize_with_type_name_opt(payload.as_slice(), type_name)
  }

  /// メッセージを指定した型にデシリアライズします。
  pub fn deserialize_message<T>(&self, message: &SerializedMessage) -> Result<T, DeserializationError>
  where
    T: DeserializeOwned, {
    postcard::from_bytes(&message.payload).map_err(|err| DeserializationError::custom(err.to_string()))
  }
}

impl Serializer for PostcardSerializer {
  fn serializer_id(&self) -> SerializerId {
    POSTCARD_SERIALIZER_ID
  }

  fn content_type(&self) -> &str {
    "application/postcard"
  }

  fn serialize_with_type_name_opt(
    &self,
    payload: &[u8],
    type_name: Option<&str>,
  ) -> Result<SerializedMessage, SerializationError> {
    // postcard は型がないと検証できない。バイト列をそのまま保持する。
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
pub fn shared_postcard_serializer() -> ArcShared<PostcardSerializer> {
  ArcShared::new(PostcardSerializer::new())
}
