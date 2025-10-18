use alloc::string::String;

use super::context_log_level::ContextLogLevel;
use crate::api::actor::{actor_id::ActorId, actor_path::ActorPath};

/// Structure that manages actor log output.
#[derive(Clone)]
pub struct ContextLogger {
  actor_id:   ActorId,
  actor_path: ActorPath,
}

impl ContextLogger {
  pub(crate) fn new(actor_id: ActorId, actor_path: &ActorPath) -> Self {
    Self { actor_id, actor_path: actor_path.clone() }
  }

  /// Gets the actor ID of the log source.
  #[must_use]
  pub const fn actor_id(&self) -> ActorId {
    self.actor_id
  }

  /// Gets the actor path of the log source.
  #[must_use]
  pub const fn actor_path(&self) -> &ActorPath {
    &self.actor_path
  }

  /// Outputs a trace level log.
  pub fn trace<F>(&self, message: F)
  where
    F: FnOnce() -> String, {
    self.emit(ContextLogLevel::Trace, message);
  }

  /// Outputs a debug level log.
  pub fn debug<F>(&self, message: F)
  where
    F: FnOnce() -> String, {
    self.emit(ContextLogLevel::Debug, message);
  }

  /// Outputs an info level log.
  pub fn info<F>(&self, message: F)
  where
    F: FnOnce() -> String, {
    self.emit(ContextLogLevel::Info, message);
  }

  /// Outputs a warn level log.
  pub fn warn<F>(&self, message: F)
  where
    F: FnOnce() -> String, {
    self.emit(ContextLogLevel::Warn, message);
  }

  /// Outputs an error level log.
  pub fn error<F>(&self, message: F)
  where
    F: FnOnce() -> String, {
    self.emit(ContextLogLevel::Error, message);
  }

  fn emit<F>(&self, level: ContextLogLevel, message: F)
  where
    F: FnOnce() -> String, {
    let text = message();

    #[cfg(feature = "tracing")]
    match level {
      | ContextLogLevel::Trace => tracing::event!(
        target: "nexus::actor",
        tracing::Level::TRACE,
        actor_id = %self.actor_id,
        actor_path = %self.actor_path,
        message = %text
      ),
      | ContextLogLevel::Debug => tracing::event!(
        target: "nexus::actor",
        tracing::Level::DEBUG,
        actor_id = %self.actor_id,
        actor_path = %self.actor_path,
        message = %text
      ),
      | ContextLogLevel::Info => tracing::event!(
        target: "nexus::actor",
        tracing::Level::INFO,
        actor_id = %self.actor_id,
        actor_path = %self.actor_path,
        message = %text
      ),
      | ContextLogLevel::Warn => tracing::event!(
        target: "nexus::actor",
        tracing::Level::WARN,
        actor_id = %self.actor_id,
        actor_path = %self.actor_path,
        message = %text
      ),
      | ContextLogLevel::Error => tracing::event!(
        target: "nexus::actor",
        tracing::Level::ERROR,
        actor_id = %self.actor_id,
        actor_path = %self.actor_path,
        message = %text
      ),
    }

    #[cfg(not(feature = "tracing"))]
    {
      let _ = self;
      let _ = &level;
      let _ = text;
    }
  }
}
