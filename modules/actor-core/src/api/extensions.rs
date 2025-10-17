mod extension;
mod registry;
mod serializer_extension;
#[cfg(test)]
mod tests;

pub use extension::{next_extension_id, Extension, ExtensionId};
pub use registry::Extensions;
pub use serializer_extension::{serializer_extension_id, SerializerRegistryExtension};
