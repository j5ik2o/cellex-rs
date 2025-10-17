use crate::api::actor::actor_failure::BehaviorFailure;
use crate::api::supervision::supervisor::supervisor_directive::SupervisorDirective;

/// Base supervisor trait.
///
/// Defines actor failure handling strategy and controls behavior on failure.
pub trait Supervisor<M>: Send + 'static {
  /// Hook called before failure handling.
  ///
  /// Default implementation does nothing.
  fn before_handle(&mut self) {}

  /// Hook called after failure handling.
  ///
  /// Default implementation does nothing.
  fn after_handle(&mut self) {}

  /// Determines the handling policy for failures.
  ///
  /// # Arguments
  ///
  /// * `_error` - Information about the error that occurred
  ///
  /// # Returns
  ///
  /// `SupervisorDirective` to execute
  fn decide(&mut self, _error: &dyn BehaviorFailure) -> SupervisorDirective {
    SupervisorDirective::Stop
  }
}
