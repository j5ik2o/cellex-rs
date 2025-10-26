use crate::v2::collections::queue::type_keys::TypeKey;

/// Marker trait for queues restricted to a single consumer.
pub trait SingleConsumer: TypeKey {}
