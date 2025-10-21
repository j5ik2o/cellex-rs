//! PID parse error type.

use core::fmt;

/// Errors that can occur while parsing a PID URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PidParseError {
  /// The URI is missing the `scheme://` delimiter or scheme component.
  MissingScheme,
  /// The URI does not contain a system identifier segment.
  MissingSystem,
  /// The node component contains an invalid port number.
  InvalidPort,
  /// One of the path segments could not be parsed into an [`ActorId`].
  InvalidPathSegment,
}

impl fmt::Display for PidParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      | Self::MissingScheme => f.write_str("missing scheme"),
      | Self::MissingSystem => f.write_str("missing system identifier"),
      | Self::InvalidPort => f.write_str("invalid node port"),
      | Self::InvalidPathSegment => f.write_str("invalid path segment"),
    }
  }
}
