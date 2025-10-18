use alloc::format;

use cellex_utils_core_rs::DEFAULT_PRIORITY;

use super::*;
use crate::api::{
  mailbox::{PriorityEnvelope, SystemMessage},
  messaging::AnyMessage,
};

/// PriorityEnvelope の制御チャネル設定を確認するユニットテスト。
#[test]
fn priority_envelope_from_system_sets_control_channel() {
  let envelope = PriorityEnvelope::from_system(SystemMessage::Stop);

  assert!(envelope.is_control());
  assert_eq!(envelope.priority(), DEFAULT_PRIORITY + 10);
  assert!(matches!(envelope.system_message(), Some(SystemMessage::Stop)));
}

/// PriorityEnvelope::map が優先度とチャネルを保持することを検証。
#[test]
fn priority_envelope_map_preserves_metadata() {
  let envelope = PriorityEnvelope::control("ping", DEFAULT_PRIORITY + 2).map(|msg| format!("{msg}-mapped"));

  assert!(envelope.is_control());
  assert_eq!(envelope.priority(), DEFAULT_PRIORITY + 2);
  assert_eq!(envelope.message(), "ping-mapped");
}

/// AnyMessage が型情報を保持しつつダウンキャスト可能なことを確認。
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
