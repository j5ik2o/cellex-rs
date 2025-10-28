//! Priority-aware envelope shared between API and internal code.

use cellex_utils_core_rs::collections::{
  queue::priority::{PriorityMessage, DEFAULT_PRIORITY},
  Element,
};

use crate::api::mailbox::messages::{PriorityChannel, SystemMessage};

/// Envelope type that stores priority and channel information for messages.
#[derive(Debug)]
pub struct PriorityEnvelope<M> {
  message:        M,
  priority:       i8,
  channel:        PriorityChannel,
  system_message: Option<SystemMessage>,
}

impl<M: Clone> Clone for PriorityEnvelope<M> {
  fn clone(&self) -> Self {
    Self {
      message:        self.message.clone(),
      priority:       self.priority,
      channel:        self.channel,
      system_message: self.system_message.clone(),
    }
  }
}

impl<M> PriorityEnvelope<M> {
  /// Creates an envelope on the regular channel with the specified priority.
  #[must_use]
  pub const fn new(message: M, priority: i8) -> Self {
    Self::with_channel(message, priority, PriorityChannel::Regular)
  }

  /// Creates an envelope with the provided priority and channel.
  #[must_use]
  pub const fn with_channel(message: M, priority: i8, channel: PriorityChannel) -> Self {
    Self { message, priority, channel, system_message: None }
  }

  /// Creates a control-channel envelope with the provided priority.
  #[must_use]
  pub const fn control(message: M, priority: i8) -> Self {
    Self::with_channel(message, priority, PriorityChannel::Control)
  }

  /// Returns a reference to the enclosed message.
  #[must_use]
  pub const fn message(&self) -> &M {
    &self.message
  }

  /// Returns the stored priority.
  #[must_use]
  pub const fn priority(&self) -> i8 {
    self.priority
  }

  /// Returns the channel where the message should be delivered.
  #[must_use]
  pub const fn channel(&self) -> PriorityChannel {
    self.channel
  }

  /// Indicates whether the message targets the control lane.
  #[must_use]
  pub const fn is_control(&self) -> bool {
    matches!(self.channel, PriorityChannel::Control)
  }

  /// Returns the associated system message when available.
  #[must_use]
  pub const fn system_message(&self) -> Option<&SystemMessage> {
    self.system_message.as_ref()
  }

  /// Decomposes the envelope into its message and priority components.
  #[must_use]
  pub fn into_parts(self) -> (M, i8) {
    (self.message, self.priority)
  }

  /// Decomposes the envelope into message, priority, and channel.
  #[must_use]
  pub fn into_parts_with_channel(self) -> (M, i8, PriorityChannel) {
    (self.message, self.priority, self.channel)
  }

  /// Maps the underlying message while preserving priority metadata.
  #[must_use]
  pub fn map<N>(self, f: impl FnOnce(M) -> N) -> PriorityEnvelope<N> {
    PriorityEnvelope {
      message:        f(self.message),
      priority:       self.priority,
      channel:        self.channel,
      system_message: self.system_message,
    }
  }

  /// Rewrites the priority using the supplied closure.
  #[must_use]
  pub fn map_priority(mut self, f: impl FnOnce(i8) -> i8) -> Self {
    self.priority = f(self.priority);
    self
  }
}

impl<M> PriorityEnvelope<M> {
  /// Creates a regular-channel envelope using the default priority.
  #[must_use]
  pub const fn with_default_priority(message: M) -> Self {
    Self::new(message, DEFAULT_PRIORITY)
  }
}

impl PriorityEnvelope<SystemMessage> {
  /// Wraps a system message while tagging the control channel and priority.
  #[must_use]
  pub fn from_system(message: SystemMessage) -> Self {
    let priority = message.priority();
    let system_clone = message.clone();
    let mut envelope = PriorityEnvelope::with_channel(message, priority, PriorityChannel::Control);
    envelope.system_message = Some(system_clone);
    envelope
  }
}

impl<M> PriorityMessage for PriorityEnvelope<M>
where
  M: Element,
{
  fn get_priority(&self) -> Option<i8> {
    Some(self.priority)
  }
}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<M> Send for PriorityEnvelope<M> where M: Send {}

#[cfg(target_has_atomic = "ptr")]
unsafe impl<M> Sync for PriorityEnvelope<M> where M: Sync {}
