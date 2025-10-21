//! Messaging primitives shared between API and internal layers.

pub mod any_message;
pub mod any_message_value;
pub mod message_envelope;

pub use any_message::AnyMessage;
pub use any_message_value::AnyMessageValue;
pub use message_envelope::MessageEnvelope;
