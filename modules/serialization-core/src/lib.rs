#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types))]

//! Core serialization abstractions shared by Nexus Actor modules.

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod error;
pub mod id;
pub mod message;
pub mod registry;
#[cfg(feature = "alloc")]
pub mod routing;
pub mod serializer;
pub mod type_key;

pub use error::{DeserializationError, RegistryError, SerializationError};
pub use id::{SerializerId, TEST_ECHO_SERIALIZER_ID, USER_DEFINED_START};
pub use message::{MessageHeader, SerializedMessage};
pub use registry::InMemorySerializerRegistry;
#[cfg(feature = "alloc")]
pub use routing::{BindingError, SerializationRouter, TypeBindingRegistry};
pub use serializer::Serializer;
pub use type_key::TypeKey;
