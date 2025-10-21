//! Type key for JSON serializer.

use cellex_serialization_core_rs::impl_type_key;

use crate::SERDE_JSON_SERIALIZER_ID;

/// Marker type representing JSON payloads within the serialization router.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonTypeKey;

impl_type_key!(JsonTypeKey, "cellex.serializer.json", SERDE_JSON_SERIALIZER_ID);
