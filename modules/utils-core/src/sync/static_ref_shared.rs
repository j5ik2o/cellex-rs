use core::ops::Deref;

use super::{Shared, SharedDyn};

/// Shared wrapper backed by a `'static` reference.
///
/// 利用者が静的領域に配置した値を `Shared` 抽象を介して扱えるようにする薄いラッパです。
pub struct StaticRefShared<T: ?Sized + 'static>(&'static T);

impl<T: ?Sized + 'static> StaticRefShared<T> {
  /// Creates a new wrapper from a `'static` reference.
  #[must_use]
  pub const fn new(reference: &'static T) -> Self {
    Self(reference)
  }

  /// Returns the raw `'static` reference.
  #[must_use]
  pub const fn as_ref(self) -> &'static T {
    self.0
  }
}

impl<T: ?Sized + 'static> Deref for StaticRefShared<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.0
  }
}

impl<T: ?Sized + 'static> Clone for StaticRefShared<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T: ?Sized + 'static> Copy for StaticRefShared<T> {}

impl<T: ?Sized + 'static> Shared<T> for StaticRefShared<T> {}

impl<T: ?Sized + 'static> SharedDyn<T> for StaticRefShared<T> {
  type Dyn<U: ?Sized + 'static> = StaticRefShared<U>;

  fn into_dyn<U: ?Sized + 'static, F>(self, cast: F) -> Self::Dyn<U>
  where
    F: FnOnce(&T) -> &U, {
    let reference = cast(self.0);
    StaticRefShared::new(reference)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  static VALUE: u32 = 42;

  #[test]
  fn deref_returns_inner_reference() {
    let shared = StaticRefShared::new(&VALUE);
    assert_eq!(*shared, 42);
  }

  #[test]
  fn clone_preserves_identity() {
    let shared = StaticRefShared::new(&VALUE);
    let cloned = shared;
    assert!(core::ptr::eq(shared.as_ref(), cloned.as_ref()));
  }

  static OTHER: (u32, u32) = (1, 2);

  #[test]
  fn into_dyn_maps_to_trait_view() {
    trait Pair {
      fn left(&self) -> u32;
    }

    impl Pair for (u32, u32) {
      fn left(&self) -> u32 {
        self.0
      }
    }

    let shared = StaticRefShared::new(&OTHER);
    let dyn_shared = shared.into_dyn(|pair| pair as &dyn Pair);
    assert_eq!(dyn_shared.left(), 1);
  }
}
