//! Shared handle to a system message mapper function.
//!
//! This module provides `MapSystemShared`, which wraps a function that converts
//! `SystemMessage` to user-defined message types, enabling type-safe system message handling.

use core::ops::Deref;

use cellex_utils_core_rs::sync::{shared::SharedBound, ArcShared};

use crate::api::mailbox::messages::SystemMessage;

#[cfg(target_has_atomic = "ptr")]
type MapSystemFn<M> = dyn Fn(SystemMessage) -> M + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
type MapSystemFn<M> = dyn Fn(SystemMessage) -> M;

/// Shared handle to a system message mapper function.
pub struct MapSystemShared<M> {
  inner: ArcShared<MapSystemFn<M>>,
}

impl<M> MapSystemShared<M> {
  /// Creates a new shared mapper from a function or closure.
  #[must_use]
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(SystemMessage) -> M + SharedBound + 'static, {
    let shared = ArcShared::new(f);
    Self { inner: shared.into_dyn(|func| func as &MapSystemFn<M>) }
  }

  /// Wraps an existing shared mapper.
  #[must_use]
  pub fn from_shared(inner: ArcShared<MapSystemFn<M>>) -> Self {
    Self { inner }
  }

  /// Consumes the wrapper and returns the underlying shared handle.
  #[must_use]
  pub fn into_shared(self) -> ArcShared<MapSystemFn<M>> {
    self.inner
  }

  /// Returns the inner shared handle.
  #[must_use]
  pub fn as_shared(&self) -> &ArcShared<MapSystemFn<M>> {
    &self.inner
  }
}

impl<M> Clone for MapSystemShared<M> {
  fn clone(&self) -> Self {
    Self { inner: self.inner.clone() }
  }
}

impl<M> Deref for MapSystemShared<M> {
  type Target = MapSystemFn<M>;

  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
