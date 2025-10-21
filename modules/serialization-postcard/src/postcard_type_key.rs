//! Type key for Postcard serializer.

use cellex_serialization_core_rs::impl_type_key;

use crate::POSTCARD_SERIALIZER_ID;

/// Marker type representing postcard-encoded payload bindings.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostcardTypeKey;

impl_type_key!(PostcardTypeKey, "cellex.serializer.postcard", POSTCARD_SERIALIZER_ID);
