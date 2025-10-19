//! Protobuf serializer implementation backed by [`prost`].

#![deny(missing_docs)]

use cellex_serialization_core_rs::{
  error::{DeserializationError, SerializationError},
  id::SerializerId,
  impl_type_key,
  message::SerializedMessage,
  serializer::Serializer,
};
use cellex_utils_core_rs::sync::ArcShared;
use prost::Message;

/// Serializer ID reserved for the `prost` backend.
pub const PROST_SERIALIZER_ID: SerializerId = SerializerId::new(2);

/// Serializer implementation backed by `prost`.
#[derive(Debug, Clone, Default)]
pub struct ProstSerializer;

impl ProstSerializer {
  /// Creates a new serializer instance.
  #[must_use]
  pub const fn new() -> Self {
    Self
  }

  /// Encodes the given message and returns it as a [`SerializedMessage`].
  pub fn serialize_message<T>(
    &self,
    type_name: Option<&str>,
    value: &T,
  ) -> Result<SerializedMessage, SerializationError>
  where
    T: Message, {
    let mut buffer = Vec::new();
    value.encode(&mut buffer).map_err(|err| SerializationError::custom(err.to_string()))?;
    self.serialize_with_type_name_opt(buffer.as_slice(), type_name)
  }

  /// Decodes the payload into the requested message type.
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
pub fn shared_prost_serializer() -> ArcShared<ProstSerializer> {
  ArcShared::new(ProstSerializer::new())
}

/// Marker type representing Prost-encoded payload bindings.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProstTypeKey;

impl_type_key!(ProstTypeKey, "cellex.serializer.prost", PROST_SERIALIZER_ID);
