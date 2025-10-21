use alloc::rc::Rc;
use core::cell::{Ref, RefCell, RefMut};

use cellex_utils_core_rs::StateCell;

/// `Rc<RefCell<T>>`-based state cell.
///
/// Provides shared mutable state using `Rc` and `RefCell` in `no_std` environments.
/// Implements the `StateCell` trait to enable the interior mutability pattern.
///
/// # Features
///
/// - Reference counting via `Rc` (single-threaded only)
/// - Interior mutability via `RefCell`
/// - Runtime borrow checking
#[derive(Debug)]
pub struct RcStateCell<T>(Rc<RefCell<T>>);

impl<T> RcStateCell<T> {
  /// Creates a new state cell with the specified value.
  pub fn new(value: T) -> Self {
    <Self as StateCell<T>>::new(value)
  }

  /// Creates a state cell from an existing `Rc<RefCell<T>>`.
  pub const fn from_rc(rc: Rc<RefCell<T>>) -> Self {
    Self(rc)
  }

  /// Extracts the inner `Rc<RefCell<T>>`.
  #[must_use]
  pub fn into_rc(self) -> Rc<RefCell<T>> {
    self.0
  }
}

impl<T> Clone for RcStateCell<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> StateCell<T> for RcStateCell<T> {
  type Ref<'a>
    = Ref<'a, T>
  where
    Self: 'a,
    T: 'a;
  type RefMut<'a>
    = RefMut<'a, T>
  where
    Self: 'a,
    T: 'a;

  fn new(value: T) -> Self
  where
    Self: Sized, {
    Self(Rc::new(RefCell::new(value)))
  }

  fn borrow(&self) -> Self::Ref<'_> {
    self.0.borrow()
  }

  fn borrow_mut(&self) -> Self::RefMut<'_> {
    self.0.borrow_mut()
  }
}
