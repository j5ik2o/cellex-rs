#![allow(clippy::disallowed_types)]

use alloc::sync::Arc;

use cellex_utils_core_rs::{MpscBuffer, QueueStorage, RingBuffer, RingBufferStorage, StateCell};
use embassy_sync::{
  blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex},
  mutex::{Mutex, MutexGuard},
};

/// `Arc`-based mutable state cell using embassy-sync's `Mutex`.
///
/// This type combines `Arc` with [`embassy_sync::mutex::Mutex`] to provide shared mutable
/// state with thread-safe reference counting. The mutex type is parameterized by a
/// [`RawMutex`] implementation, allowing different synchronization strategies:
///
/// - [`NoopRawMutex`]: No synchronization (single-threaded or cooperative multitasking)
/// - [`CriticalSectionRawMutex`]: Uses critical sections for interrupt safety
///
/// # Type Parameters
///
/// - `T`: The type of value stored in the cell
/// - `RM`: The raw mutex implementation (defaults to [`NoopRawMutex`])
///
/// # Examples
///
/// ```
/// use cellex_utils_embedded_rs::sync::ArcLocalStateCell;
///
/// let cell = ArcLocalStateCell::new(0);
/// let clone = cell.clone();
///
/// *clone.borrow_mut() = 42;
/// assert_eq!(*cell.borrow(), 42);
/// ```
#[derive(Debug)]
pub struct ArcStateCell<T, RM = NoopRawMutex>
where
  RM: RawMutex, {
  inner: Arc<Mutex<RM, T>>,
}

/// Type alias for `ArcStateCell` with [`NoopRawMutex`].
///
/// This variant provides no synchronization and is suitable for single-threaded
/// environments or cooperative multitasking where the executor guarantees non-preemption.
pub type ArcLocalStateCell<T> = ArcStateCell<T, NoopRawMutex>;

/// Type alias for `ArcStateCell` with [`CriticalSectionRawMutex`].
///
/// This variant uses critical sections for synchronization, making it safe to use
/// across interrupts and in preemptive multitasking environments.
pub type ArcCsStateCell<T> = ArcStateCell<T, CriticalSectionRawMutex>;

impl<T, RM> ArcStateCell<T, RM>
where
  RM: RawMutex,
{
  /// Creates a new `ArcStateCell` containing the given value.
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::sync::ArcLocalStateCell;
  ///
  /// let cell = ArcLocalStateCell::new(42);
  /// assert_eq!(*cell.borrow(), 42);
  /// ```
  pub fn new(value: T) -> Self {
    Self { inner: Arc::new(Mutex::new(value)) }
  }

  /// Creates a new `ArcStateCell` from an existing `Arc<Mutex<RM, T>>`.
  ///
  /// This allows wrapping an already-allocated mutex without additional allocation.
  ///
  /// # Examples
  ///
  /// ```
  /// use alloc::sync::Arc;
  ///
  /// use cellex_utils_embedded_rs::sync::ArcStateCell;
  /// use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
  ///
  /// let arc = Arc::new(Mutex::<NoopRawMutex, _>::new(42));
  /// let cell = ArcStateCell::from_arc(arc);
  /// assert_eq!(*cell.borrow(), 42);
  /// ```
  pub fn from_arc(inner: Arc<Mutex<RM, T>>) -> Self {
    Self { inner }
  }

  /// Consumes this `ArcStateCell` and returns the underlying `Arc<Mutex<RM, T>>`.
  ///
  /// # Examples
  ///
  /// ```
  /// use cellex_utils_embedded_rs::sync::ArcLocalStateCell;
  ///
  /// let cell = ArcLocalStateCell::new(42);
  /// let arc = cell.into_arc();
  /// assert_eq!(*arc.try_lock().unwrap(), 42);
  /// ```
  pub fn into_arc(self) -> Arc<Mutex<RM, T>> {
    self.inner
  }

  fn lock(&self) -> MutexGuard<'_, RM, T> {
    self.inner.try_lock().unwrap_or_else(|_| panic!("ArcStateCell: concurrent access detected"))
  }
}

impl<T, RM> Clone for ArcStateCell<T, RM>
where
  RM: RawMutex,
{
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl<T, RM> StateCell<T> for ArcStateCell<T, RM>
where
  RM: RawMutex,
{
  type Ref<'a>
    = MutexGuard<'a, RM, T>
  where
    Self: 'a,
    T: 'a;
  type RefMut<'a>
    = MutexGuard<'a, RM, T>
  where
    Self: 'a,
    T: 'a;

  fn new(value: T) -> Self
  where
    Self: Sized, {
    ArcStateCell::new(value)
  }

  fn borrow(&self) -> Self::Ref<'_> {
    self.lock()
  }

  fn borrow_mut(&self) -> Self::RefMut<'_> {
    self.lock()
  }
}

impl<T, RM> RingBufferStorage<T> for ArcStateCell<MpscBuffer<T>, RM>
where
  RM: RawMutex,
{
  fn with_read<R>(&self, f: impl FnOnce(&MpscBuffer<T>) -> R) -> R {
    let guard = self.borrow();
    f(&guard)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut MpscBuffer<T>) -> R) -> R {
    let mut guard = self.borrow_mut();
    f(&mut guard)
  }
}

impl<E, RM> QueueStorage<E> for ArcStateCell<RingBuffer<E>, RM>
where
  RM: RawMutex,
{
  fn with_read<R>(&self, f: impl FnOnce(&RingBuffer<E>) -> R) -> R {
    let guard = self.lock();
    f(&guard)
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut RingBuffer<E>) -> R) -> R {
    let mut guard = self.lock();
    f(&mut guard)
  }
}
