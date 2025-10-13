//! Utilities for declaring logical type keys used during serialization routing.

use crate::id::SerializerId;

/// Trait implemented by message types that can be serialized through the router.
///
/// Each implementer provides a globally unique key that is used to resolve the
/// appropriate serializer at runtime. Optionally, a default serializer identifier
/// can be exposed so that registry extensions are able to install sensible
/// bindings automatically.
pub trait TypeKey: 'static {
  /// Returns the canonical string key associated with the implementing type.
  fn type_key() -> &'static str;

  /// Returns the default serializer identifier, when one is recommended.
  ///
  /// Implementers should override this method when a specific serializer is
  /// expected for the type. The default implementation returns `None`.
  #[inline]
  fn default_serializer() -> Option<SerializerId> {
    None
  }
}

/// Declares a [`TypeKey`] implementation for the provided type.
///
/// # Examples
///
/// ```ignore
/// use cellex_serialization_core_rs::{impl_type_key, SerializerId, TypeKey};
///
/// struct Example;
///
/// impl_type_key!(Example);
/// assert_eq!(<Example as TypeKey>::type_key(), core::any::type_name::<Example>());
///
/// struct JsonPayload;
///
/// impl_type_key!(JsonPayload, "example.JsonPayload", SerializerId::new(1));
/// assert_eq!(<JsonPayload as TypeKey>::type_key(), "example.JsonPayload");
/// assert_eq!(<JsonPayload as TypeKey>::default_serializer(), Some(SerializerId::new(1)));
/// ```
#[macro_export]
macro_rules! impl_type_key {
  ($ty:ty) => {
    impl $crate::TypeKey for $ty {
      fn type_key() -> &'static str {
        core::any::type_name::<$ty>()
      }
    }
  };
  ($ty:ty, $key:expr) => {
    impl $crate::TypeKey for $ty {
      fn type_key() -> &'static str {
        $key
      }
    }
  };
  ($ty:ty, $key:expr, $serializer:expr) => {
    impl $crate::TypeKey for $ty {
      fn type_key() -> &'static str {
        $key
      }

      fn default_serializer() -> Option<$crate::SerializerId> {
        Some($serializer)
      }
    }
  };
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{impl_type_key, SerializerId};

  struct Basic;

  impl_type_key!(Basic);

  struct Custom;

  impl_type_key!(Custom, "custom.Type");

  struct WithSerializer;

  impl_type_key!(WithSerializer, "custom.Serializer", SerializerId::new(99));

  #[test]
  fn derives_type_name_by_default() {
    assert_eq!(<Basic as TypeKey>::type_key(), core::any::type_name::<Basic>());
    assert_eq!(<Basic as TypeKey>::default_serializer(), None);
  }

  #[test]
  fn overrides_key_when_specified() {
    assert_eq!(<Custom as TypeKey>::type_key(), "custom.Type");
  }

  #[test]
  fn exposes_default_serializer_when_requested() {
    assert_eq!(
      <WithSerializer as TypeKey>::default_serializer(),
      Some(SerializerId::new(99))
    );
  }
}
