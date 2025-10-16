mod internal_message_metadata;
mod internal_message_sender;
mod message_envelope;
mod message_metadata;
mod message_sender;
mod user_message;

use crate::{DynMessage, PriorityEnvelope};
use cellex_utils_core_rs::QueueError;
pub use internal_message_metadata::InternalMessageMetadata;
pub use internal_message_sender::InternalMessageSender;
pub use message_envelope::MessageEnvelope;
pub use message_metadata::MessageMetadata;
pub use message_sender::MessageSender;
pub use user_message::UserMessage;

#[cfg(target_has_atomic = "ptr")]
type SendFn = dyn Fn(DynMessage, i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type SendFn = dyn Fn(DynMessage, i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>;

#[cfg(target_has_atomic = "ptr")]
type DropHookFn = dyn Fn() + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type DropHookFn = dyn Fn();
