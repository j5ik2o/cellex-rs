use core::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use super::node::WaitNode;
use crate::sync::ArcShared;

/// Future returned when registering interest in a queue/stack event.
pub struct WaitHandle<E: Copy> {
  node: ArcShared<WaitNode<E>>,
}

impl<E: Copy> WaitHandle<E> {
  /// Creates a wait handle bound to the supplied waiter node.
  pub fn new(node: ArcShared<WaitNode<E>>) -> Self {
    Self { node }
  }

  fn node(&self) -> &WaitNode<E> {
    &self.node
  }
}

impl<E: Copy> Future for WaitHandle<E> {
  type Output = Result<(), E>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.node().poll(cx)
  }
}

impl<E: Copy> Drop for WaitHandle<E> {
  fn drop(&mut self) {
    self.node.cancel();
  }
}

impl<E: Copy> Clone for WaitHandle<E> {
  fn clone(&self) -> Self {
    Self { node: self.node.clone() }
  }
}
