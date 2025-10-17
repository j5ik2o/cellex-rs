/// Actor lifecycle signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
  /// Signal sent after the actor stops.
  PostStop,
}
