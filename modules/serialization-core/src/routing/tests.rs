use alloc::vec::Vec;

use cellex_utils_core_rs::sync::ArcShared;

use super::*;
use crate::{
  error::{DeserializationError, SerializationError},
  message::SerializedMessage,
  registry::InMemorySerializerRegistry,
  routing::BindingError,
  Serializer, SerializerId,
};

#[derive(Clone, Debug)]
struct EchoSerializer;

impl Serializer for EchoSerializer {
  fn serializer_id(&self) -> SerializerId {
    SerializerId::new(42)
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
      message.set_type_name(name);
    }
    Ok(message)
  }

  fn deserialize(&self, message: &SerializedMessage) -> Result<Vec<u8>, DeserializationError> {
    Ok(message.payload.clone())
  }
}

#[test]
fn binds_and_resolves_serializer() {
  let bindings = TypeBindingRegistry::new();
  let serializers = InMemorySerializerRegistry::new();
  let router = SerializationRouter::new(bindings.clone(), serializers.clone());

  let serializer = ArcShared::new(EchoSerializer);
  serializers.register(serializer).expect("serializer");
  bindings.bind("example.Type", SerializerId::new(42)).expect("bind");

  let resolved = router.resolve_serializer("example.Type").expect("resolve");
  assert_eq!(resolved.serializer_id(), SerializerId::new(42));

  let message = resolved.serialize_with_type_name_opt(b"hello", Some("example.Type")).expect("serialize");
  assert_eq!(message.payload, b"hello");
}

#[test]
fn resolve_or_fallback_uses_fallback_serializer() {
  let bindings = TypeBindingRegistry::new();
  let serializers = InMemorySerializerRegistry::new();
  let router = SerializationRouter::with_fallback(bindings.clone(), serializers.clone(), Some(SerializerId::new(42)));

  let serializer = ArcShared::new(EchoSerializer);
  serializers.register(serializer).expect("serializer");

  assert!(router.resolve_serializer("missing.Type").is_none());

  let resolved = router.resolve_or_fallback("missing.Type").expect("fallback");
  assert_eq!(resolved.serializer_id(), SerializerId::new(42));

  let message = resolved.serialize_with_type_name_opt(b"hello", Some("missing.Type")).expect("serialize");
  assert_eq!(message.payload, b"hello");

  let fallback = router.fallback_serializer().expect("fallback serializer");
  assert_eq!(fallback.serializer_id(), SerializerId::new(42));
}

#[test]
fn duplicate_binding_fails() {
  let bindings = TypeBindingRegistry::new();
  bindings.bind("dup.Type", SerializerId::new(1)).expect("bind first");
  let err = bindings.bind("dup.Type", SerializerId::new(2)).expect_err("duplicate");
  assert!(matches!(err, BindingError::DuplicateBinding(key) if key == "dup.Type"));
}
