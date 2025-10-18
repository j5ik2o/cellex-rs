use core::any::Any;

/// Type bound required for values stored inside [`DynMessage`].
/// Trait bound describing values that can be erased into [`DynMessage`].
#[cfg(target_has_atomic = "ptr")]
pub trait DynMessageValue: Any + Send + Sync {}

/// Trait bound describing values that can be erased into [`DynMessage`] on targets without atomic
/// pointers.
#[cfg(not(target_has_atomic = "ptr"))]
pub trait DynMessageValue: Any {}

#[cfg(target_has_atomic = "ptr")]
impl<T> DynMessageValue for T where T: Any + Send + Sync {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> DynMessageValue for T where T: Any {}
