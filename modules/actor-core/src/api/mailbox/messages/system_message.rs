use cellex_utils_core_rs::DEFAULT_PRIORITY;

use crate::api::{actor::ActorId, supervision::failure::FailureInfo};

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
      | SystemMessage::Watch(_) | SystemMessage::Unwatch(_) => DEFAULT_PRIORITY + 5,
      | SystemMessage::Stop => DEFAULT_PRIORITY + 10,
      | SystemMessage::Failure(_) => DEFAULT_PRIORITY + 12,
      | SystemMessage::Restart => DEFAULT_PRIORITY + 11,
      | SystemMessage::Suspend | SystemMessage::Resume => DEFAULT_PRIORITY + 9,
      | SystemMessage::Escalate(_) => DEFAULT_PRIORITY + 13,
      | SystemMessage::ReceiveTimeout => DEFAULT_PRIORITY + 8,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::actor::actor_failure::ActorFailure;

  #[test]
  fn priority_table_matches_expected_values() {
    let base = DEFAULT_PRIORITY;
    let failure_info = FailureInfo::new(
      crate::api::actor::ActorId(1),
      crate::api::actor::ActorPath::new(),
      ActorFailure::from_message("fail"),
    );

    let expectations = [
      (SystemMessage::Watch(crate::api::actor::ActorId(1)), base + 5),
      (SystemMessage::Unwatch(crate::api::actor::ActorId(1)), base + 5),
      (SystemMessage::Stop, base + 10),
      (SystemMessage::Failure(failure_info.clone()), base + 12),
      (SystemMessage::Restart, base + 11),
      (SystemMessage::Suspend, base + 9),
      (SystemMessage::Resume, base + 9),
      (SystemMessage::Escalate(failure_info.clone()), base + 13),
      (SystemMessage::ReceiveTimeout, base + 8),
    ];

    for (message, expected) in expectations {
      assert_eq!(message.priority(), expected, "unexpected priority for {message:?}");
    }
  }
}
