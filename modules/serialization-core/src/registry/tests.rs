use alloc::vec::Vec;

use super::*;
use crate::{
  error::{DeserializationError, SerializationError},
  id::{SerializerId, TEST_ECHO_SERIALIZER_ID},
  message::SerializedMessage,
};

#[derive(Debug)]
struct EchoSerializer;

impl Serializer for EchoSerializer {
  fn serializer_id(&self) -> SerializerId {
    TEST_ECHO_SERIALIZER_ID
  }

  fn content_type(&self) -> &str {
    "application/octet-stream"
  }

  fn serialize_with_type_name_opt(
    &self,
    payload: &[u8],
    type_name: Option<&str>,
  ) -> Result<SerializedMessage, SerializationError> {
    let mut message = SerializedMessage::new(self.serializer_id(), payload.to_vec());
    if let Some(name) = type_name {
      message = message.with_type_name(name);
    }
    Ok(message)
  }

  fn deserialize(&self, message: &SerializedMessage) -> Result<Vec<u8>, DeserializationError> {
    Ok(message.payload.clone())
  }
}

#[test]
fn registers_and_resolves_serializer() {
  let registry = InMemorySerializerRegistry::new();
  let serializer = ArcShared::new(EchoSerializer);
  registry.register(serializer.clone()).expect("register");

  let resolved = registry.get(TEST_ECHO_SERIALIZER_ID).expect("resolve");
  assert_eq!(resolved.serializer_id(), TEST_ECHO_SERIALIZER_ID);

  let serialized = resolved.serialize_with_type_name(b"ping", "Example").expect("serialize");
  let payload = resolved.deserialize(&serialized).expect("deserialize");
  assert_eq!(payload, b"ping");
}

#[test]
fn rejects_duplicate_ids() {
  let registry = InMemorySerializerRegistry::new();
  let first = ArcShared::new(EchoSerializer);
  let second = ArcShared::new(EchoSerializer);

  registry.register(first).expect("register first");
  let err = registry.register(second).expect_err("duplicate");
  assert!(matches!(err, RegistryError::DuplicateEntry(id) if id == TEST_ECHO_SERIALIZER_ID));
}
