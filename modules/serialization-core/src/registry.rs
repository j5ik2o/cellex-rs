//! In-memory serializer registry implementation.

#[cfg(all(feature = "alloc", test))]
mod tests;

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;

#[cfg(feature = "alloc")]
use cellex_utils_core_rs::sync::ArcShared;
#[cfg(feature = "alloc")]
use spin::RwLock;

#[cfg(feature = "alloc")]
use crate::error::RegistryError;
#[cfg(feature = "alloc")]
use crate::id::SerializerId;
#[cfg(feature = "alloc")]
use crate::serializer::Serializer;

/// Default registry backed by a thread-safe map of serializer identifiers.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct InMemorySerializerRegistry {
  inner: ArcShared<RwLock<BTreeMap<SerializerId, ArcShared<dyn Serializer>>>>,
}

#[cfg(feature = "alloc")]
impl InMemorySerializerRegistry {
  /// Creates a new, empty registry.
  #[must_use]
  pub fn new() -> Self {
    Self { inner: ArcShared::new(RwLock::new(BTreeMap::new())) }
  }

  fn insert_serializer<S>(&self, serializer: ArcShared<S>) -> Result<(), RegistryError>
  where
    S: Serializer + 'static, {
    let serializer_id = serializer.serializer_id();
    let mut guard = self.inner.write();
    if guard.contains_key(&serializer_id) {
      return Err(RegistryError::DuplicateEntry(serializer_id));
    }
    let trait_obj = serializer.into_dyn(|ext| ext as &dyn Serializer);
    guard.insert(serializer_id, trait_obj);
    Ok(())
  }

  /// Registers a serializer implementation.
  pub fn register<S>(&self, serializer: ArcShared<S>) -> Result<(), RegistryError>
  where
    S: Serializer + 'static, {
    self.insert_serializer(serializer)
  }

  /// Registers a trait-object serializer.
  pub fn register_trait(&self, serializer: ArcShared<dyn Serializer>) -> Result<(), RegistryError> {
    let serializer_id = serializer.serializer_id();
    let mut guard = self.inner.write();
    if guard.contains_key(&serializer_id) {
      return Err(RegistryError::DuplicateEntry(serializer_id));
    }
    guard.insert(serializer_id, serializer);
    Ok(())
  }

  /// Retrieves the serializer associated with the provided identifier.
  #[must_use]
  pub fn get(&self, serializer_id: SerializerId) -> Option<ArcShared<dyn Serializer>> {
    self.inner.read().get(&serializer_id).cloned()
  }
}

#[cfg(feature = "alloc")]
impl Default for InMemorySerializerRegistry {
  fn default() -> Self {
    Self::new()
  }
}
