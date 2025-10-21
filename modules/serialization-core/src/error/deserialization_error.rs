//! Deserialization error type.

#[cfg(feature = "alloc")]
use alloc::string::String;

/// Error returned when a serialized payload cannot be converted back to its logical representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeserializationError {
  /// The payload contains malformed data.
  MalformedPayload,
  /// The serializer implementation reported a custom error.
  #[cfg(feature = "alloc")]
  Custom(String),
}

impl DeserializationError {
  #[cfg(feature = "alloc")]
  /// Constructs a custom deserialization error from the provided message.
  #[must_use]
  pub fn custom(message: String) -> Self {
    DeserializationError::Custom(message)
  }
}

impl core::fmt::Display for DeserializationError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      | DeserializationError::MalformedPayload => f.write_str("malformed payload"),
      #[cfg(feature = "alloc")]
      | DeserializationError::Custom(message) => f.write_str(message),
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for DeserializationError {}
