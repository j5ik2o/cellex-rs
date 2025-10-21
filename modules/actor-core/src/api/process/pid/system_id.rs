//! System identifier type.

use alloc::borrow::Cow;
use core::fmt;

/// Identifier of the actor system namespace.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SystemId(pub Cow<'static, str>);

impl SystemId {
  /// Creates a new [`SystemId`] from the provided string.
  #[must_use]
  pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
    Self(id.into())
  }
}

impl fmt::Display for SystemId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}
