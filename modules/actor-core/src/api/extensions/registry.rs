use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};

use cellex_utils_core_rs::sync::ArcShared;
use spin::RwLock;

use super::extension::{Extension, ExtensionId};

/// Container managing registered extensions.
pub struct Extensions {
  slots: ArcShared<RwLock<Vec<Option<ArcShared<dyn Extension>>>>>,
}

impl Extensions {
  /// Creates an empty extension registry.
  #[must_use]
  pub fn new() -> Self {
    Self { slots: ArcShared::new(RwLock::new(Vec::new())) }
  }

  /// Builds a registry from an externally managed slot list.
  #[must_use]
  pub fn from_shared(slots: ArcShared<RwLock<Vec<Option<ArcShared<dyn Extension>>>>>) -> Self {
    Self { slots }
  }

  /// Registers an extension implementing [`Extension`].
  pub fn register<E>(&self, extension: ArcShared<E>)
  where
    E: Extension + 'static, {
    let id = extension.extension_id();
    if id < 0 {
      return;
    }
    let handle = extension.into_dyn(|ext| ext as &dyn Extension);
    self.store_extension(id, handle);
  }

  /// Registers a type-erased extension.
  pub fn register_dyn(&self, extension: ArcShared<dyn Extension>) {
    let id = extension.extension_id();
    if id < 0 {
      return;
    }
    self.store_extension(id, extension);
  }

  /// Retrieves an extension by identifier.
  #[must_use]
  pub fn get(&self, id: ExtensionId) -> Option<ArcShared<dyn Extension>> {
    if id < 0 {
      return None;
    }
    let guard = self.slots.read();
    guard.get(id as usize).and_then(|slot| slot.as_ref().cloned())
  }

  /// Applies a closure to the extension of type `E` stored at the provided identifier.
  pub fn with<E, F, R>(&self, id: ExtensionId, f: F) -> Option<R>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> R, {
    let guard = self.slots.read();
    guard.get(id as usize).and_then(|slot| slot.as_ref().and_then(|handle| handle.as_any().downcast_ref::<E>().map(f)))
  }

  fn store_extension(&self, id: ExtensionId, extension: ArcShared<dyn Extension>) {
    let idx = id as usize;
    let mut guard = self.slots.write();
    if guard.len() <= idx {
      guard.resize_with(idx + 1, || None);
    }
    guard[idx] = Some(extension);
  }
}

impl Default for Extensions {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for Extensions {
  fn clone(&self) -> Self {
    Self { slots: self.slots.clone() }
  }
}

impl Debug for Extensions {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    let guard = self.slots.read();
    f.debug_struct("Extensions").field("len", &guard.len()).finish()
  }
}
