//! Process resolution type.

use core::fmt;

use cellex_utils_core_rs::sync::ArcShared;

/// Result of resolving a PID within the registry.
#[derive(Clone)]
pub enum ProcessResolution<T> {
  /// The PID maps to a local process handle.
  Local(ArcShared<T>),
  /// The PID belongs to a remote node.
  Remote,
  /// No process is registered for the PID.
  Unresolved,
}

impl<T> fmt::Debug for ProcessResolution<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      | Self::Local(_) => f.write_str("Local(..)"),
      | Self::Remote => f.write_str("Remote"),
      | Self::Unresolved => f.write_str("Unresolved"),
    }
  }
}
