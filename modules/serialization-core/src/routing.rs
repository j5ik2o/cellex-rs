//! Type binding registry and routing utilities.

#![cfg(feature = "alloc")]

use alloc::collections::btree_map::{BTreeMap, Entry};
use alloc::string::String;

use crate::id::SerializerId;
use crate::registry::InMemorySerializerRegistry;
use crate::serializer::Serializer;
use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

/// Errors that can occur when manipulating type bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingError {
  /// A binding for the provided type key already exists.
  DuplicateBinding(String),
}

impl core::fmt::Display for BindingError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      BindingError::DuplicateBinding(key) => write!(f, "type key '{key}' is already bound"),
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for BindingError {}

/// Registry that maps logical type keys to serializer identifiers.
#[derive(Clone)]
pub struct TypeBindingRegistry {
  inner: ArcShared<RwLock<BTreeMap<String, SerializerId>>>,
}

impl TypeBindingRegistry {
  /// Creates an empty binding registry.
  #[must_use]
  pub fn new() -> Self {
    Self {
      inner: ArcShared::new(RwLock::new(BTreeMap::new())),
    }
  }

  /// Registers a new binding. Returns an error if the key is already in use.
  pub fn bind<K>(&self, key: K, serializer: SerializerId) -> Result<(), BindingError>
  where
    K: Into<String>, {
    let key_string = key.into();
    let mut guard = self.inner.write();
    match guard.entry(key_string.clone()) {
      Entry::Occupied(_) => Err(BindingError::DuplicateBinding(key_string)),
      Entry::Vacant(vacant) => {
        vacant.insert(serializer);
        Ok(())
      }
    }
  }

  /// Removes a binding, returning the previous serializer identifier if present.
  pub fn unbind(&self, key: &str) -> Option<SerializerId> {
    self.inner.write().remove(key)
  }

  /// Resolves the serializer identifier associated with the provided key.
  #[must_use]
  pub fn resolve(&self, key: &str) -> Option<SerializerId> {
    self.inner.read().get(key).copied()
  }

  /// Returns `true` if a binding exists for the specified key.
  #[must_use]
  pub fn contains(&self, key: &str) -> bool {
    self.inner.read().contains_key(key)
  }
}

impl Default for TypeBindingRegistry {
  fn default() -> Self {
    Self::new()
  }
}

/// Routing facade that combines type bindings with the serializer registry.
#[derive(Clone)]
pub struct SerializationRouter {
  bindings: TypeBindingRegistry,
  serializers: InMemorySerializerRegistry,
}

impl SerializationRouter {
  /// Creates a new router backed by the provided binding registry and serializer registry.
  #[must_use]
  pub fn new(bindings: TypeBindingRegistry, serializers: InMemorySerializerRegistry) -> Self {
    Self { bindings, serializers }
  }

  /// Returns a clone of the binding registry.
  #[must_use]
  pub fn bindings(&self) -> TypeBindingRegistry {
    self.bindings.clone()
  }

  /// Returns a clone of the serializer registry.
  #[must_use]
  pub fn serializers(&self) -> InMemorySerializerRegistry {
    self.serializers.clone()
  }

  /// Resolves the serializer associated with the provided type key.
  #[must_use]
  pub fn resolve_serializer(&self, type_key: &str) -> Option<ArcShared<dyn Serializer>> {
    let serializer_id = self.bindings.resolve(type_key)?;
    self.serializers.get(serializer_id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::{DeserializationError, SerializationError};
  use crate::message::SerializedMessage;
  use alloc::vec::Vec;

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

    let message = resolved
      .serialize_with_type_name_opt(b"hello", Some("example.Type"))
      .expect("serialize");
    assert_eq!(message.payload, b"hello");
  }

  #[test]
  fn duplicate_binding_fails() {
    let bindings = TypeBindingRegistry::new();
    bindings.bind("dup.Type", SerializerId::new(1)).expect("bind first");
    let err = bindings.bind("dup.Type", SerializerId::new(2)).expect_err("duplicate");
    assert!(matches!(err, BindingError::DuplicateBinding(key) if key == "dup.Type"));
  }
}
