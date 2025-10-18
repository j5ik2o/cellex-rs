/// Context log level representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextLogLevel {
  /// Trace level logging.
  Trace,
  /// Debug level logging.
  Debug,
  /// Info level logging.
  Info,
  /// Warn level logging.
  Warn,
  /// Error level logging.
  Error,
}
