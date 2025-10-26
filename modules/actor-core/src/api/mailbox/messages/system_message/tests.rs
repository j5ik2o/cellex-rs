#![allow(clippy::disallowed_types)]
use cellex_utils_core_rs::collections::queue::priority::DEFAULT_PRIORITY;

use super::SystemMessage;
use crate::api::{
  actor::{actor_failure::ActorFailure, ActorId, ActorPath},
  failure::FailureInfo,
};

#[test]
fn priority_table_matches_expected_values() {
  let base = DEFAULT_PRIORITY;
  let failure_info = FailureInfo::new(ActorId(1), ActorPath::new(), ActorFailure::from_message("fail"));

  let expectations = [
    (SystemMessage::Watch(ActorId(1)), base + 5),
    (SystemMessage::Unwatch(ActorId(1)), base + 5),
    (SystemMessage::Stop, base + 10),
    (SystemMessage::Failure(failure_info.clone()), base + 12),
    (SystemMessage::Restart, base + 11),
    (SystemMessage::Suspend, base + 9),
    (SystemMessage::Resume, base + 9),
    (SystemMessage::Escalate(failure_info), base + 13),
    (SystemMessage::ReceiveTimeout, base + 8),
  ];

  for (message, expected) in expectations {
    assert_eq!(message.priority(), expected, "unexpected priority for {message:?}");
  }
}
