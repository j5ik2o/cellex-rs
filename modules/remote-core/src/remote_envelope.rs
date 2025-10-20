use cellex_actor_core_rs::api::mailbox::messages::{PriorityChannel, PriorityEnvelope};

/// Transport-level wrapper that preserves priority and channel information across remote hops.
#[derive(Debug, Clone)]
pub struct RemoteEnvelope<M> {
  priority: i8,
  channel:  PriorityChannel,
  message:  M,
}

impl<M> RemoteEnvelope<M> {
  /// Creates a new `RemoteEnvelope` with the specified message, priority, and channel.
  pub const fn new(message: M, priority: i8, channel: PriorityChannel) -> Self {
    Self { priority, channel, message }
  }

  /// Returns the stored priority.
  pub const fn priority(&self) -> i8 {
    self.priority
  }

  /// Returns the channel classification for the message.
  pub const fn channel(&self) -> PriorityChannel {
    self.channel
  }

  /// Returns a reference to the enclosed message payload.
  pub const fn message(&self) -> &M {
    &self.message
  }

  /// Decomposes the envelope into `(message, priority, channel)`.
  #[allow(clippy::missing_const_for_fn)]
  pub fn into_parts(self) -> (M, i8, PriorityChannel) {
    (self.message, self.priority, self.channel)
  }

  /// Alias for [`Self::into_parts`] to emphasize channel preservation.
  #[allow(clippy::missing_const_for_fn)]
  pub fn into_parts_with_channel(self) -> (M, i8, PriorityChannel) {
    self.into_parts()
  }
}

impl<M> From<PriorityEnvelope<M>> for RemoteEnvelope<M> {
  fn from(envelope: PriorityEnvelope<M>) -> Self {
    let (message, priority, channel) = envelope.into_parts_with_channel();
    Self::new(message, priority, channel)
  }
}

impl<M> From<RemoteEnvelope<M>> for PriorityEnvelope<M> {
  fn from(envelope: RemoteEnvelope<M>) -> Self {
    let (message, priority, channel) = envelope.into_parts();
    PriorityEnvelope::with_channel(message, priority, channel)
  }
}
