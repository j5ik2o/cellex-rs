pub mod internal_message_metadata;
pub mod internal_message_sender;
mod metadata_table;
mod metadata_table_inner;

pub use metadata_table::{discard_metadata, store_metadata, take_metadata, MetadataKey};
pub(crate) use metadata_table_inner::MetadataTableInner;
