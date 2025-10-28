#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types))]

//! Core serialization abstractions shared by Nexus Actor modules.

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod error;
mod id;
pub mod message;
mod registry;
#[cfg(feature = "alloc")]
pub mod routing;
mod serializer;
mod type_key;

pub use id::*;
pub use registry::*;
pub use serializer::*;
pub use type_key::*;
