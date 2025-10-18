#[cfg(test)]
mod tests;

use core::task::Poll;
use std::collections::HashMap;

use cellex_utils_core_rs::{
  DeadlineTimer, DeadlineTimerError, DeadlineTimerExpired, DeadlineTimerKey, DeadlineTimerKeyAllocator, TimerDeadline,
};
use tokio_util::time::delay_queue::{DelayQueue as InnerDelayQueue, Key as InnerKey};

/// Implementation that wraps `tokio_util::time::DelayQueue` to satisfy the `DeadlineTimer`
/// abstraction.
///
/// Maintains forward and reverse conversion tables to avoid exposing the internal keys of
/// DelayQueue externally. Consistently exchanges `DeadlineTimerKey` with the core layer.
/// Used as a common foundation for deadline-based processing on the Tokio runtime, including
/// `ReceiveTimeout`.
#[derive(Debug)]
pub struct TokioDeadlineTimer<Item> {
  inner:     InnerDelayQueue<Item>,
  allocator: DeadlineTimerKeyAllocator,
  forward:   HashMap<DeadlineTimerKey, InnerKey>,
  reverse:   HashMap<InnerKey, DeadlineTimerKey>,
}

impl<Item> TokioDeadlineTimer<Item> {
  /// Creates an empty DeadlineTimer.
  #[inline]
  pub fn new() -> Self {
    Self::with_inner(InnerDelayQueue::new())
  }

  /// Creates a DeadlineTimer with pre-allocated capacity.
  #[inline]
  pub fn with_capacity(capacity: usize) -> Self {
    Self::with_inner(InnerDelayQueue::with_capacity(capacity))
  }

  fn with_inner(inner: InnerDelayQueue<Item>) -> Self {
    Self { inner, allocator: DeadlineTimerKeyAllocator::new(), forward: HashMap::new(), reverse: HashMap::new() }
  }

  fn release_key_mapping(&mut self, inner_key: &InnerKey) -> Option<DeadlineTimerKey> {
    if let Some(key) = self.reverse.remove(inner_key) {
      self.forward.remove(&key);
      Some(key)
    } else {
      None
    }
  }
}

impl<Item> Default for TokioDeadlineTimer<Item> {
  fn default() -> Self {
    Self::new()
  }
}

impl<Item> DeadlineTimer for TokioDeadlineTimer<Item> {
  type Error = DeadlineTimerError;
  type Item = Item;

  fn insert(&mut self, item: Self::Item, deadline: TimerDeadline) -> Result<DeadlineTimerKey, Self::Error> {
    let key = self.allocator.allocate();
    let inner_key = self.inner.insert(item, deadline.as_duration());
    self.forward.insert(key, inner_key);
    self.reverse.insert(inner_key, key);
    Ok(key)
  }

  fn reset(&mut self, key: DeadlineTimerKey, deadline: TimerDeadline) -> Result<(), Self::Error> {
    let inner_key = *self.forward.get(&key).ok_or(DeadlineTimerError::KeyNotFound)?;
    self.inner.reset(&inner_key, deadline.as_duration());
    Ok(())
  }

  fn cancel(&mut self, key: DeadlineTimerKey) -> Result<Option<Self::Item>, Self::Error> {
    if let Some(inner_key) = self.forward.remove(&key) {
      self.reverse.remove(&inner_key);
      let removed = self.inner.remove(&inner_key);
      Ok(Some(removed.into_inner()))
    } else {
      Ok(None)
    }
  }

  fn poll_expired(
    &mut self,
    cx: &mut core::task::Context<'_>,
  ) -> Poll<Result<DeadlineTimerExpired<Self::Item>, Self::Error>> {
    match self.inner.poll_expired(cx) {
      | Poll::Ready(Some(expired)) => {
        let inner_key = expired.key();
        let item = expired.into_inner();
        if let Some(key) = self.release_key_mapping(&inner_key) {
          Poll::Ready(Ok(DeadlineTimerExpired { key, item }))
        } else {
          Poll::Ready(Err(DeadlineTimerError::KeyNotFound))
        }
      },
      | Poll::Ready(None) => Poll::Ready(Err(DeadlineTimerError::Closed)),
      | Poll::Pending => Poll::Pending,
    }
  }
}
