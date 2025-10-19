use alloc::string::String;
use core::{any::Any, sync::atomic::Ordering};

use cellex_serialization_core_rs::{
  BindingError, InMemorySerializerRegistry, RegistryError, SerializationRouter, Serializer, SerializerId,
  TypeBindingRegistry, TypeKey,
};
#[cfg(feature = "std")]
use cellex_serialization_json_rs::{shared_json_serializer, JsonTypeKey, SERDE_JSON_SERIALIZER_ID};
#[cfg(feature = "postcard")]
use cellex_serialization_postcard_rs::{shared_postcard_serializer, PostcardTypeKey, POSTCARD_SERIALIZER_ID};
#[cfg(feature = "std")]
use cellex_serialization_prost_rs::{shared_prost_serializer, ProstTypeKey, PROST_SERIALIZER_ID};
use cellex_utils_core_rs::ArcShared;
use portable_atomic::AtomicI32;

use super::extension::{next_extension_id, Extension, ExtensionId};

static SERIALIZER_EXTENSION_ID: AtomicI32 = AtomicI32::new(-1);

pub(crate) fn acquire_serializer_extension_id() -> ExtensionId {
  let current = SERIALIZER_EXTENSION_ID.load(Ordering::SeqCst);
  if current >= 0 {
    return current;
  }
  let new_id = next_extension_id();
  match SERIALIZER_EXTENSION_ID.compare_exchange(-1, new_id, Ordering::SeqCst, Ordering::SeqCst) {
    | Ok(_) => new_id,
    | Err(existing) => existing,
  }
}

/// Returns the reserved extension identifier for the serializer registry.
#[must_use]
pub fn serializer_extension_id() -> ExtensionId {
  acquire_serializer_extension_id()
}

/// Extension that exposes the shared serializer registry.
pub struct SerializerRegistryExtension {
  id:       ExtensionId,
  registry: InMemorySerializerRegistry,
  bindings: TypeBindingRegistry,
  router:   SerializationRouter,
}

impl SerializerRegistryExtension {
  /// Creates a new registry extension and installs built-in serializers.
  #[must_use]
  pub fn new() -> Self {
    let registry = InMemorySerializerRegistry::new();
    let bindings = TypeBindingRegistry::new();
    let router = SerializationRouter::new(bindings.clone(), registry.clone());
    let extension = Self { id: serializer_extension_id(), registry, bindings, router };
    extension.install_builtin_serializers();
    extension.install_default_bindings();
    extension
  }

  fn install_builtin_serializers(&self) {
    #[cfg(feature = "std")]
    {
      if self.registry.get(SERDE_JSON_SERIALIZER_ID).is_none() {
        let serializer = shared_json_serializer();
        let _ = self.registry.register(serializer);
      }
      if self.registry.get(PROST_SERIALIZER_ID).is_none() {
        let serializer = shared_prost_serializer();
        let _ = self.registry.register(serializer);
      }
    }
    #[cfg(feature = "postcard")]
    {
      if self.registry.get(POSTCARD_SERIALIZER_ID).is_none() {
        let serializer = shared_postcard_serializer();
        let _ = self.registry.register(serializer);
      }
    }
  }

  fn install_default_bindings(&self) {
    #[cfg(feature = "std")]
    {
      let _ = self.bind_type::<JsonTypeKey>(SERDE_JSON_SERIALIZER_ID);
      let _ = self.bind_type::<ProstTypeKey>(PROST_SERIALIZER_ID);
    }
    #[cfg(feature = "postcard")]
    {
      let _ = self.bind_type::<PostcardTypeKey>(POSTCARD_SERIALIZER_ID);
    }
  }

  /// Returns a reference to the underlying registry.
  #[must_use]
  pub fn registry(&self) -> &InMemorySerializerRegistry {
    &self.registry
  }

  /// Returns the binding registry used by the router.
  #[must_use]
  pub fn bindings(&self) -> &TypeBindingRegistry {
    &self.bindings
  }

  /// Returns a serialization router instance backed by the shared registries.
  #[must_use]
  pub fn router(&self) -> SerializationRouter {
    self.router.clone()
  }

  /// Registers a serializer implementation, returning an error when the ID clashes.
  ///
  /// # Errors
  /// Returns [`RegistryError`] when an identical serializer identifier is already registered.
  pub fn register_serializer<S>(&self, serializer: ArcShared<S>) -> Result<(), RegistryError>
  where
    S: Serializer + 'static, {
    self.registry.register(serializer)
  }

  /// Binds the provided key to the specified serializer identifier.
  ///
  /// # Errors
  /// Returns [`BindingError`] when the key cannot be associated with the serializer identifier.
  pub fn bind_key<K>(&self, key: K, serializer: SerializerId) -> Result<(), BindingError>
  where
    K: Into<String>, {
    self.bindings.bind(key, serializer)
  }

  /// Binds the [`TypeKey::KEY`] of `T` to the specified serializer identifier.
  ///
  /// # Errors
  /// Returns [`BindingError`] when the type key binding fails.
  pub fn bind_type<T>(&self, serializer: SerializerId) -> Result<(), BindingError>
  where
    T: TypeKey, {
    self.bind_key(<T as TypeKey>::type_key(), serializer)
  }

  /// Binds `T` using its [`TypeKey::default_serializer`] when available.
  ///
  /// # Errors
  /// Returns [`BindingError`] when binding the type key fails or no default serializer is
  /// available.
  pub fn bind_type_with_default<T>(&self) -> Result<(), BindingError>
  where
    T: TypeKey, {
    if let Some(serializer) = T::default_serializer() {
      self.bind_type::<T>(serializer)
    } else {
      Ok(())
    }
  }
}

impl Extension for SerializerRegistryExtension {
  fn extension_id(&self) -> ExtensionId {
    self.id
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
