//! PID tag type.

use alloc::borrow::Cow;
use core::fmt;

/// Optional tag associated with a PID (e.g. incarnation).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PidTag(pub Cow<'static, str>);

impl PidTag {
  /// Creates a new tag.
  #[must_use]
  pub fn new(tag: impl Into<Cow<'static, str>>) -> Self {
    Self(tag.into())
  }
}

impl fmt::Display for PidTag {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}
