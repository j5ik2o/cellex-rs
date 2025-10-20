#![allow(clippy::disallowed_types)]
use alloc::format;

use cellex_utils_core_rs::DEFAULT_PRIORITY;

use super::*;
use crate::api::{
  mailbox::messages::{PriorityEnvelope, SystemMessage},
  messaging::AnyMessage,
};

/// Ensures `PriorityEnvelope::from_system` marks the envelope as a control message.
#[test]
fn priority_envelope_from_system_sets_control_channel() {
  let envelope = PriorityEnvelope::from_system(SystemMessage::Stop);

  assert!(envelope.is_control());
  assert_eq!(envelope.priority(), DEFAULT_PRIORITY + 10);
  assert!(matches!(envelope.system_message(), Some(SystemMessage::Stop)));
}

/// Ensures `PriorityEnvelope::map` preserves priority and channel metadata.
#[test]
fn priority_envelope_map_preserves_metadata() {
  let envelope = PriorityEnvelope::control("ping", DEFAULT_PRIORITY + 2).map(|msg| format!("{msg}-mapped"));

  assert!(envelope.is_control());
  assert_eq!(envelope.priority(), DEFAULT_PRIORITY + 2);
  assert_eq!(envelope.message(), "ping-mapped");
}

/// Verifies that `AnyMessage` retains type information and can be downcast.
#[test]
fn dyn_message_downcast_recovers_value() {
  let message = AnyMessage::new(42_u32);

  assert_eq!(message.type_id(), core::any::TypeId::of::<u32>());
}

#[cfg(target_has_atomic = "ptr")]
#[test]
fn dyn_message_is_send_sync_static() {
  fn assert_bounds<T: Send + Sync + 'static>() {}
  assert_bounds::<AnyMessage>();
}

#[cfg(target_has_atomic = "ptr")]
#[test]
fn priority_envelope_is_send_sync_static() {
  fn assert_bounds<T: Send + Sync + 'static>() {}
  assert_bounds::<PriorityEnvelope<SystemMessage>>();
}
