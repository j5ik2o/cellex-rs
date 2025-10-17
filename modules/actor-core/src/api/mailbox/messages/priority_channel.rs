/// Channel type that distinguishes regular messages from control traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorityChannel {
  /// Regular application messages.
  Regular,
  /// System control messages (stop, restart, etc.).
  Control,
}
