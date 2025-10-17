mod internal_message_metadata;
mod internal_message_sender;
mod metadata_table;
mod metadata_table_inner;

pub(crate) use internal_message_metadata::InternalMessageMetadata;
pub(crate) use internal_message_sender::InternalMessageSender;
pub use metadata_table::{discard_metadata, store_metadata, take_metadata, MetadataKey};
#[allow(unused_imports)]
pub(crate) use metadata_table_inner::MetadataTableInner;
