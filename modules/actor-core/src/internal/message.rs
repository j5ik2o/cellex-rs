pub mod internal_message_metadata;
pub mod internal_message_sender;
pub mod metadata_table;

pub use crate::api::messaging::DynMessage;
pub use metadata_table::{discard_metadata, store_metadata, take_metadata, MetadataKey};
