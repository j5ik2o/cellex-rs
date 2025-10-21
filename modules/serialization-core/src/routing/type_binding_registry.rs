//! Type binding registry type.

use alloc::{
  collections::btree_map::{BTreeMap, Entry},
  string::String,
};

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use crate::{id::SerializerId, routing::BindingError};

/// Registry that maps logical type keys to serializer identifiers.
#[derive(Clone)]
pub struct TypeBindingRegistry {
  inner: ArcShared<RwLock<BTreeMap<String, SerializerId>>>,
}

impl TypeBindingRegistry {
  /// Creates an empty binding registry.
  #[must_use]
  pub fn new() -> Self {
    Self { inner: ArcShared::new(RwLock::new(BTreeMap::new())) }
  }

  /// Registers a new binding. Returns an error if the key is already in use.
  pub fn bind<K>(&self, key: K, serializer: SerializerId) -> Result<(), BindingError>
  where
    K: Into<String>, {
    let key_string = key.into();
    let mut guard = self.inner.write();
    match guard.entry(key_string.clone()) {
      | Entry::Occupied(_) => Err(BindingError::DuplicateBinding(key_string)),
      | Entry::Vacant(vacant) => {
        vacant.insert(serializer);
        Ok(())
      },
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
