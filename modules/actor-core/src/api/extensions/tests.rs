extern crate alloc;

#[cfg(feature = "std")]
use alloc::string::String;
use core::any::Any;

#[cfg(feature = "std")]
use cellex_serialization_core_rs::{impl_type_key, TypeKey};
#[cfg(feature = "std")]
use cellex_serialization_json_rs::{JsonTypeKey, SERDE_JSON_SERIALIZER_ID};
#[cfg(feature = "postcard")]
use cellex_serialization_postcard_rs::{PostcardTypeKey, POSTCARD_SERIALIZER_ID};
#[cfg(feature = "std")]
use cellex_serialization_prost_rs::{ProstTypeKey, PROST_SERIALIZER_ID};
use cellex_utils_core_rs::sync::ArcShared;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use serde_json;

use super::*;

#[derive(Debug)]
struct DummyExtension {
  id:    ExtensionId,
  value: usize,
}

impl DummyExtension {
  fn new(value: usize) -> Self {
    Self { id: next_extension_id(), value }
  }
}

impl Extension for DummyExtension {
  fn extension_id(&self) -> ExtensionId {
    self.id
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[test]
fn register_and_lookup_extension() {
  let extensions = Extensions::new();
  let extension = ArcShared::new(DummyExtension::new(42));
  let id = extension.extension_id();
  extensions.register(extension.clone());

  let stored = extensions.get(id).expect("extension should exist");
  assert_eq!(stored.extension_id(), id);

  let value = extensions.with::<DummyExtension, _, _>(id, |ext| ext.value).expect("typed borrow");
  assert_eq!(value, 42);
}

#[cfg(feature = "std")]
#[test]
fn serializer_extension_installs_default_bindings() {
  let extension = SerializerRegistryExtension::new();
  let router = extension.router();

  let serializer = router.resolve_serializer(<JsonTypeKey as TypeKey>::type_key()).expect("json binding should exist");
  assert_eq!(serializer.serializer_id(), SERDE_JSON_SERIALIZER_ID);

  let serializer =
    router.resolve_serializer(<ProstTypeKey as TypeKey>::type_key()).expect("prost binding should exist");
  assert_eq!(serializer.serializer_id(), PROST_SERIALIZER_ID);
}

#[cfg(all(feature = "std", feature = "postcard"))]
#[test]
fn serializer_extension_installs_postcard_binding() {
  let extension = SerializerRegistryExtension::new();
  let router = extension.router();

  let serializer =
    router.resolve_serializer(<PostcardTypeKey as TypeKey>::type_key()).expect("postcard binding should exist");
  assert_eq!(serializer.serializer_id(), POSTCARD_SERIALIZER_ID);
}

#[cfg(feature = "std")]
#[test]
fn router_round_trip_serializes_and_deserializes_payload() {
  #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
  struct JsonPayload {
    message: String,
    count:   u32,
  }

  impl_type_key!(JsonPayload, "test.JsonPayload");

  let extension = SerializerRegistryExtension::new();
  extension.bind_type::<JsonPayload>(SERDE_JSON_SERIALIZER_ID).expect("bind payload type");

  let router = extension.router();
  let serializer = router.resolve_serializer(<JsonPayload as TypeKey>::type_key()).expect("registered serializer");

  let payload = JsonPayload { message: "hello".to_owned(), count: 7 };
  let encoded = serde_json::to_vec(&payload).expect("encode json");

  let message = serializer
    .serialize_with_type_name_opt(encoded.as_slice(), Some(<JsonPayload as TypeKey>::type_key()))
    .expect("serialize");
  assert_eq!(message.serializer_id, SERDE_JSON_SERIALIZER_ID);
  assert_eq!(message.type_name.as_deref(), Some(<JsonPayload as TypeKey>::type_key()));

  let decoded = serializer.deserialize(&message).expect("deserialize");
  let recovered: JsonPayload = serde_json::from_slice(&decoded).expect("decode json");
  assert_eq!(recovered, payload);
}
