//! Type key for Prost serializer.

use cellex_serialization_core_rs::impl_type_key;

use crate::PROST_SERIALIZER_ID;

/// Marker type representing Prost-encoded payload bindings.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProstTypeKey;

impl_type_key!(ProstTypeKey, "cellex.serializer.prost", PROST_SERIALIZER_ID);
