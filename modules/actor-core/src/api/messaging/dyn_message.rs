use alloc::boxed::Box;
use core::{
  any::{Any, TypeId},
  fmt::{self, Debug},
};

use super::dyn_message_value::DynMessageValue;

#[cfg(target_has_atomic = "ptr")]
type DynMessageInner = dyn Any + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type DynMessageInner = dyn Any;

/// Type-erased message used across the public API.
pub struct DynMessage {
  inner: Box<DynMessageInner>,
}

impl DynMessage {
  /// Creates a `DynMessage` wrapping an arbitrary value.
  pub fn new<T>(value: T) -> Self
  where
    T: DynMessageValue + 'static, {
    Self { inner: Box::new(value) }
  }

  /// Gets the `TypeId` of the internally held value.
  pub fn type_id(&self) -> TypeId {
    self.inner.as_ref().type_id()
  }

  /// Attempts to downcast to type `T` by moving ownership.
  pub fn downcast<T>(self) -> Result<T, Self>
  where
    T: DynMessageValue + 'static, {
    match self.inner.downcast::<T>() {
      | Ok(boxed) => Ok(*boxed),
      | Err(inner) => Err(Self { inner }),
    }
  }

  /// Attempts to downcast to type `T` through a shared reference.
  pub fn downcast_ref<T>(&self) -> Option<&T>
  where
    T: DynMessageValue + 'static, {
    self.inner.downcast_ref::<T>()
  }

  /// Attempts to downcast to type `T` through a mutable reference.
  pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
  where
    T: DynMessageValue + 'static, {
    self.inner.downcast_mut::<T>()
  }

  /// Extracts the internal type-erased value.
  pub fn into_any(self) -> Box<DynMessageInner> {
    self.inner
  }
}

impl Debug for DynMessage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "DynMessage<{}>", core::any::type_name::<Self>())
  }
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl Send for DynMessage {}

#[cfg(target_has_atomic = "ptr")]
unsafe impl Sync for DynMessage {}

#[cfg(target_has_atomic = "ptr")]
const fn assert_send_dyn<T: Send>() {}

#[cfg(target_has_atomic = "ptr")]
const fn assert_sync_dyn<T: Sync>() {}

#[cfg(target_has_atomic = "ptr")]
const _: () = {
  assert_send_dyn::<DynMessage>();
  assert_sync_dyn::<DynMessage>();
  assert_static_dyn::<DynMessage>();
};

#[cfg(target_has_atomic = "ptr")]
const fn assert_static_dyn<T: 'static>() {}
