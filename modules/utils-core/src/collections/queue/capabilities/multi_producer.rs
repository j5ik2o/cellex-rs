use crate::collections::queue::type_keys::TypeKey;

/// Marker trait for queues supporting multiple producers.
pub trait MultiProducer: TypeKey {}
