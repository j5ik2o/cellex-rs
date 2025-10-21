//! Messaging primitives shared between API and internal layers.

pub mod any_message; // allow module_wiring::no_parent_reexport
pub mod any_message_value; // allow module_wiring::no_parent_reexport
pub mod map_system; // allow module_wiring::no_parent_reexport
pub mod message_envelope; // allow module_wiring::no_parent_reexport

pub use any_message::AnyMessage;
pub use any_message_value::AnyMessageValue;
pub use map_system::MapSystemShared;
pub use message_envelope::MessageEnvelope;
