//! Registry error type.

/// Errors that can occur while modifying a serializer registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
  /// A serializer with the same identifier has already been registered.
  DuplicateEntry(crate::id::SerializerId),
}

impl core::fmt::Display for RegistryError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | RegistryError::DuplicateEntry(id) => write!(f, "serializer id {id} already registered"),
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for RegistryError {}
