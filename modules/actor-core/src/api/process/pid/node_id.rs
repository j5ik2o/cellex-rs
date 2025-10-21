//! Node identifier type.

use alloc::borrow::Cow;
use core::fmt;

/// Unique identifier of the node within a cluster.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
  host: Cow<'static, str>,
  port: Option<u16>,
}

impl NodeId {
  /// Creates a new [`NodeId`] with host and optional port.
  #[must_use]
  pub fn new(host: impl Into<Cow<'static, str>>, port: Option<u16>) -> Self {
    Self { host: host.into(), port }
  }

  /// Returns the host name.
  #[must_use]
  pub fn host(&self) -> &str {
    &self.host
  }

  /// Returns the port, if specified.
  #[must_use]
  pub const fn port(&self) -> Option<u16> {
    self.port
  }
}

impl fmt::Display for NodeId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.port {
      | Some(port) => write!(f, "{}:{}", self.host, port),
      | None => f.write_str(&self.host),
    }
  }
}
