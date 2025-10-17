use crate::api::actor::ActorId;
use crate::api::supervision::failure::FailureInfo;
use cellex_utils_core_rs::DEFAULT_PRIORITY;

/// Control message types inspired by protoactor-go's `SystemMessage` catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemMessage {
  /// Start watching another actor.
  Watch(ActorId),
  /// Stop watching another actor.
  Unwatch(ActorId),
  /// Instruct the actor to stop.
  Stop,
  /// Notify of a failure occurrence.
  Failure(FailureInfo),
  /// Request the actor to restart.
  Restart,
  /// Suspend actor message processing.
  Suspend,
  /// Resume actor message processing.
  Resume,
  /// Escalate a failure to the parent actor.
  Escalate(FailureInfo),
  /// Notify that the receive timeout elapsed.
  ReceiveTimeout,
}

impl SystemMessage {
  /// Returns the recommended runtime priority for the system message.
  pub fn priority(&self) -> i8 {
    match self {
      SystemMessage::Watch(_) | SystemMessage::Unwatch(_) => DEFAULT_PRIORITY + 5,
      SystemMessage::Stop => DEFAULT_PRIORITY + 10,
      SystemMessage::Failure(_) => DEFAULT_PRIORITY + 12,
      SystemMessage::Restart => DEFAULT_PRIORITY + 11,
      SystemMessage::Suspend | SystemMessage::Resume => DEFAULT_PRIORITY + 9,
      SystemMessage::Escalate(_) => DEFAULT_PRIORITY + 13,
      SystemMessage::ReceiveTimeout => DEFAULT_PRIORITY + 8,
    }
  }
}
