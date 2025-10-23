use super::QueueBackend;

/// Extension trait for backends supporting priority semantics.
pub trait PriorityBackend<T: Ord>: QueueBackend<T> {
  /// Returns a reference to the smallest element without removing it.
  fn peek_min(&self) -> Option<&T>;
}
