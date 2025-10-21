//! Serialization router type.

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use crate::{
  id::SerializerId, registry::InMemorySerializerRegistry, routing::TypeBindingRegistry, serializer::Serializer,
};

/// Routing facade that combines type bindings with the serializer registry.
#[derive(Clone)]
pub struct SerializationRouter {
  bindings:    TypeBindingRegistry,
  serializers: InMemorySerializerRegistry,
  fallback:    ArcShared<RwLock<Option<SerializerId>>>,
}

impl SerializationRouter {
  /// Creates a new router backed by the provided binding registry and serializer registry.
  #[must_use]
  pub fn new(bindings: TypeBindingRegistry, serializers: InMemorySerializerRegistry) -> Self {
    Self::with_fallback(bindings, serializers, None)
  }

  /// Creates a new router with an optional fallback serializer identifier.
  #[must_use]
  pub fn with_fallback(
    bindings: TypeBindingRegistry,
    serializers: InMemorySerializerRegistry,
    fallback: Option<SerializerId>,
  ) -> Self {
    let fallback_handle = ArcShared::new(RwLock::new(fallback));
    Self { bindings, serializers, fallback: fallback_handle }
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

  /// Returns the serializer configured as fallback when a type binding is missing.
  #[must_use]
  pub fn fallback_serializer(&self) -> Option<ArcShared<dyn Serializer>> {
    let fallback = *self.fallback.read();
    fallback.and_then(|serializer_id| self.serializers.get(serializer_id))
  }

  /// Resolves a serializer for the type key, returning the fallback when no binding exists.
  #[must_use]
  pub fn resolve_or_fallback(&self, type_key: &str) -> Option<ArcShared<dyn Serializer>> {
    self.resolve_serializer(type_key).or_else(|| {
      let fallback = *self.fallback.read();
      fallback.and_then(|serializer_id| self.serializers.get(serializer_id))
    })
  }

  /// Updates the fallback serializer identifier used when no type binding exists.
  pub fn set_fallback_serializer(&self, fallback: Option<SerializerId>) {
    *self.fallback.write() = fallback;
  }
}
