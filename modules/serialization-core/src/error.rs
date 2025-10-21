//! Error types used across the serialization core.

mod deserialization_error;
mod registry_error;
mod serialization_error;

pub use deserialization_error::DeserializationError;
pub use registry_error::RegistryError;
pub use serialization_error::SerializationError;
