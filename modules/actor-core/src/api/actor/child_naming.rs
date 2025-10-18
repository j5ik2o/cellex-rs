use alloc::string::String;

/// Naming strategy applied when spawning a child actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChildNaming {
  /// Automatically assign an incrementing identifier-based name.
  Auto,
  /// Generate a name using the provided prefix followed by a unique suffix.
  WithPrefix(String),
  /// Use the provided name verbatim; fails if the name already exists.
  Explicit(String),
}

impl Default for ChildNaming {
  fn default() -> Self {
    Self::Auto
  }
}
