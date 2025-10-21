#![allow(clippy::disallowed_types)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use alloc::string::String;

use crate::{
  api::mailbox::{messages::PriorityChannel, ThreadSafe},
  shared::messaging::message_envelope::MessageEnvelope,
};

#[test]
fn into_priority_envelope_retains_priority() {
  let message = MessageEnvelope::user(String::from("ping"));
  let envelope = message.into_priority_envelope(7);
  assert_eq!(envelope.priority(), 7);
  assert_eq!(envelope.channel(), PriorityChannel::Regular);
}

#[test]
fn into_control_envelope_sets_control_channel() {
  let message: MessageEnvelope<ThreadSafe> =
    MessageEnvelope::System(crate::api::mailbox::messages::SystemMessage::Stop);
  let envelope = message.into_control_envelope(crate::api::mailbox::messages::SystemMessage::Stop.priority());
  assert_eq!(envelope.channel(), PriorityChannel::Control);
  assert_eq!(envelope.priority(), crate::api::mailbox::messages::SystemMessage::Stop.priority());
}

#[test]
fn control_user_creates_control_priority_envelope() {
  let envelope = MessageEnvelope::control_user(String::from("urgent"), 10);
  assert_eq!(envelope.priority(), 10);
  assert_eq!(envelope.channel(), PriorityChannel::Control);
  let (message, _, _) = envelope.into_parts_with_channel();
  match message {
    | MessageEnvelope::User(user) => {
      let (value, metadata) = user.into_parts::<ThreadSafe>();
      assert_eq!(value, "urgent");
      assert!(metadata.is_none());
    },
    | MessageEnvelope::System(_) => panic!("expected user envelope"),
  }
}
