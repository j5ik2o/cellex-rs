/// Execution state tracked by [`ActorCell`].
pub(crate) enum ActorCellState {
  /// Actor is actively processing messages.
  Running,
  /// Actor is suspended and blocks user messages.
  Suspended,
  /// Actor has terminated and will not process further work.
  Stopped,
}
