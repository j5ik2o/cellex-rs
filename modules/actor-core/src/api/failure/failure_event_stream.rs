mod failure_event_listener_shared;
#[cfg(all(test, feature = "std"))]
pub(crate) mod tests;

use cellex_utils_core_rs::sync::{SendBound, SharedBound};
pub use failure_event_listener_shared::*;

/// Listener for receiving failure events as a stream.
///
/// Subscribes to failure events from the entire actor system and executes custom processing.
pub type FailureEventListener = FailureEventListenerShared;

/// Stream abstraction for distributing FailureEvent externally.
///
/// Implementations are placed in peripheral crates like `actor-std` or `actor-embedded`,
/// and are used from `actor-core` via dependency inversion.
pub trait FailureEventStream: Clone + SharedBound + 'static {
  /// Handle type representing a subscription. Handles cleanup like unsubscribing on Drop.
  type Subscription: SendBound + 'static;

  /// Returns a listener to receive FailureEvent notifications.
  fn listener(&self) -> FailureEventListener;

  /// Registers a new subscriber and returns a subscription handle.
  fn subscribe(&self, listener: FailureEventListener) -> Self::Subscription;
}
