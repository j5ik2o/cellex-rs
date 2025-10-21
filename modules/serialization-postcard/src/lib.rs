//! Serializer implementation backed by the `postcard` format.

#![no_std]
#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types))]

extern crate alloc;

use alloc::{string::ToString, vec::Vec};

mod postcard_type_key;

use cellex_serialization_core_rs::{
  DeserializationError, SerializationError, SerializedMessage, Serializer, SerializerId,
};
use cellex_utils_core_rs::sync::ArcShared;
pub use postcard_type_key::PostcardTypeKey;
use serde::{de::DeserializeOwned, Serialize};

/// Serializer ID reserved for the `postcard` backend.
pub const POSTCARD_SERIALIZER_ID: SerializerId = SerializerId::new(3);

/// Serializer implementation backed by `postcard`.
#[derive(Debug, Clone, Default)]
pub struct PostcardSerializer;

impl PostcardSerializer {
  /// Creates a new serializer instance.
  #[must_use]
  pub const fn new() -> Self {
    Self
  }

  /// Encodes the given value and returns it as a [`SerializedMessage`].
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

  /// Decodes the payload into the requested type.
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
    // Validation requires the caller to supply the concrete message type, so the binary is stored
    // as-is.
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

/// Returns a shared serializer handle.
#[must_use]
pub fn shared_postcard_serializer() -> ArcShared<PostcardSerializer> {
  ArcShared::new(PostcardSerializer::new())
}
