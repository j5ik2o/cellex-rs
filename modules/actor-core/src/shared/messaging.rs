//! Messaging primitives shared between API and internal layers.

mod any_message;
mod any_message_value;
mod map_system;
mod message_envelope;

pub use any_message::AnyMessage;
pub use any_message_value::AnyMessageValue;
pub use map_system::MapSystemShared;
pub use message_envelope::MessageEnvelope;
