//! Message header type.

#[cfg(feature = "alloc")]
use alloc::string::String;

/// Key/value metadata attached to a serialized payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageHeader {
  /// Header name.
  #[cfg(feature = "alloc")]
  pub key:   String,
  /// Header value.
  #[cfg(feature = "alloc")]
  pub value: String,
}

impl MessageHeader {
  /// Creates a new header entry.
  #[cfg(feature = "alloc")]
  #[inline]
  #[must_use]
  pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
    Self { key: key.into(), value: value.into() }
  }
}
