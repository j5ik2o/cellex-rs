mod base;
mod extension;
mod serializer_extension;
#[cfg(test)]
mod tests;

pub use base::Extensions;
pub use extension::{next_extension_id, Extension, ExtensionId};
pub use serializer_extension::{serializer_extension_id, SerializerRegistryExtension};
