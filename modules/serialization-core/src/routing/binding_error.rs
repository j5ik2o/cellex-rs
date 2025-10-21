//! Type binding error type.

use alloc::string::String;

/// Errors that can occur when manipulating type bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingError {
  /// A binding for the provided type key already exists.
  DuplicateBinding(String),
}

impl core::fmt::Display for BindingError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | BindingError::DuplicateBinding(key) => write!(f, "type key '{key}' is already bound"),
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for BindingError {}
