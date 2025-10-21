use cellex_actor_core_rs::api::mailbox::messages::SystemMessage;
use cellex_serialization_core_rs::SerializedMessage;

/// Payload variants for remote transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemotePayloadFrame {
  /// System message payload retained as-is.
  System(SystemMessage),
  /// User message encoded via the serialization layer.
  User {
    /// Serialized representation produced by the configured serializer.
    serialized: SerializedMessage,
  },
}
